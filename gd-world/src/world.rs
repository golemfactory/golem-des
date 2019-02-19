use std::collections::HashMap;

use gd_engine::Engine;
use log::debug;
use rand::seq::SliceRandom;

use crate::id::Id;
use crate::provider;
use crate::provider::Provider;
use crate::requestor;
use crate::requestor::Requestor;
use crate::task::*;

#[derive(Debug)]
pub enum Event {
    TaskAdvertisement(Id),
    SubTaskComputed(SubTask, Id, Id, f64),
    SubTaskBudgetExceeded(SubTask, Id, Id),
}

#[derive(Debug)]
pub struct World<Rng>
where
    Rng: rand::Rng,
{
    rng: Rng,
    engine: Engine<Event>,
    requestors: HashMap<Id, Requestor>,
    providers: HashMap<Id, Box<dyn Provider>>,
}

impl<Rng> World<Rng>
where
    Rng: rand::Rng,
{
    pub fn new(rng: Rng) -> World<Rng> {
        World {
            rng: rng,
            engine: Engine::new(),
            requestors: HashMap::new(),
            providers: HashMap::new(),
        }
    }

    pub fn push_requestor(&mut self, requestor: Requestor) {
        debug!("W:adding {}", requestor);

        self.requestors.insert(*requestor.id(), requestor);
    }

    pub fn append_requestors<It>(&mut self, requestors: It)
    where
        It: IntoIterator<Item = Requestor>,
    {
        for requestor in requestors {
            self.push_requestor(requestor);
        }
    }

    pub fn push_provider(&mut self, provider: Box<dyn Provider>) {
        debug!("W:adding {}", provider);

        self.providers.insert(*provider.id(), provider);
    }

    pub fn append_providers<It>(&mut self, providers: It)
    where
        It: IntoIterator<Item = Box<dyn Provider>>,
    {
        for provider in providers {
            self.push_provider(provider);
        }
    }

    pub fn into_stats(mut self, run_id: u64) -> (Vec<requestor::Stats>, Vec<provider::Stats>) {
        (
            self.requestors
                .drain()
                .map(|(_, requestor)| requestor.into_stats(run_id))
                .collect(),
            self.providers
                .drain()
                .map(|(_, provider)| provider.into_stats(run_id))
                .collect(),
        )
    }

    pub fn run(&mut self, until: f64) {
        self.started();

        while let Some(payload) = self.engine.next() {
            let now = self.engine.now();

            if until < now {
                break;
            }

            debug!("W:now = {}", now);
            self.handle(payload);
        }

        self.stopped();
    }

    fn handle_advertise(&mut self, requestor_id: Id) {
        let requestor = self
            .requestors
            .get_mut(&requestor_id)
            .expect("W:requestor not found!");

        // collect offers
        let mut bids = Vec::new();
        for (&id, provider) in &mut self.providers {
            if let Some(bid) = provider.send_offer() {
                bids.push((id, bid));
            }
        }

        // select offers
        for (provider_id, subtask, bid) in requestor.select_offers(bids) {
            let provider = self
                .providers
                .get_mut(&provider_id)
                .expect("W:provider not found!");

            provider.receive_subtask(&mut self.engine, &mut self.rng, &subtask, &requestor_id, bid);
        }
    }

    fn handle_compute(&mut self, subtask: SubTask, requestor_id: Id, provider_id: Id, bid: f64) {
        let requestor = self
            .requestors
            .get_mut(&requestor_id)
            .expect("W:requestor not found!");

        let provider = self
            .providers
            .get_mut(&provider_id)
            .expect("W:provider not found");

        provider.finish_computing(self.engine.now(), &subtask, &requestor_id);
        let reported_usage = provider.report_usage(&subtask, bid);
        requestor.verify_subtask(&subtask, &provider_id, bid, reported_usage);
        let payment = requestor.send_payment(&subtask, &provider_id, bid, reported_usage);
        provider.receive_payment(&subtask, &requestor_id, payment);
        requestor.complete_task();

        self.schedule_advertise();
    }

    fn handle_budget_exceeded(&mut self, subtask: SubTask, requestor_id: Id, provider_id: Id) {
        let requestor = self
            .requestors
            .get_mut(&requestor_id)
            .expect("W:requestor not found");

        let provider = self
            .providers
            .get_mut(&provider_id)
            .expect("W:provider not found");

        provider.cancel_computing(self.engine.now(), &subtask, &requestor_id);
        requestor.budget_exceeded(&subtask, &provider_id);

        self.schedule_advertise();
    }

    fn handle(&mut self, event: Event) {
        match event {
            Event::TaskAdvertisement(requestor_id) => self.handle_advertise(requestor_id),
            Event::SubTaskComputed(subtask, requestor_id, provider_id, bid) => {
                self.handle_compute(subtask, requestor_id, provider_id, bid)
            }
            Event::SubTaskBudgetExceeded(subtask, requestor_id, provider_id) => {
                self.handle_budget_exceeded(subtask, requestor_id, provider_id)
            }
        }
    }

    fn started(&mut self) {
        // collect benchmarks
        let usage_factors: Vec<(Id, f64)> = self
            .providers
            .iter()
            .map(|(&id, provider)| (id, provider.send_benchmark()))
            .collect();

        for (_, requestor) in &mut self.requestors {
            for (id, usage_factor) in &usage_factors {
                requestor.receive_benchmark(id, *usage_factor);
            }
        }

        self.schedule_advertise();

        debug!("W:simulation started");
    }

    fn stopped(&self) {
        debug!("W:simulation stopped");

        for (_, requestor) in &self.requestors {
            debug!("W:{}", requestor);
        }

        for (_, provider) in &self.providers {
            debug!("W:{}", provider);
        }
    }

    fn schedule_advertise(&mut self) {
        // shuffle requestors
        let mut ids: Vec<Id> = self.requestors.keys().cloned().collect();
        ids.shuffle(&mut self.rng);

        for id in ids {
            self.requestors
                .get_mut(&id)
                .expect("W:requestor not found")
                .advertise(&mut self.engine, &mut self.rng);
        }
    }
}
