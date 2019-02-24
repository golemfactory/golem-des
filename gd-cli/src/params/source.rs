use gd_world::prelude::*;
use gd_world::requestor::DefenceMechanismType;
use serde_derive::Deserialize;

use super::{Generator, ProviderBehaviour};

#[derive(Debug, Deserialize)]
pub struct RequestorSource {
    requestor_count: usize,
    max_price: Generator,
    budget_factor: Generator,
    subtask_count: Generator,
    nominal_usage: Generator,
}

impl RequestorSource {
    pub fn iter<'a, Rng>(
        &'a self,
        rng: &'a mut Rng,
        defence_mechanism_type: DefenceMechanismType,
    ) -> RequestorSourceIter<Rng>
    where
        Rng: rand::Rng,
    {
        RequestorSourceIter {
            count: 0,
            source: self,
            rng,
            defence_mechanism_type,
        }
    }
}

#[derive(Debug)]
pub struct RequestorSourceIter<'a, Rng>
where
    Rng: rand::Rng,
{
    count: usize,
    source: &'a RequestorSource,
    rng: &'a mut Rng,
    defence_mechanism_type: DefenceMechanismType,
}

impl<'a, Rng> Iterator for RequestorSourceIter<'a, Rng>
where
    Rng: rand::Rng,
{
    type Item = Requestor;

    fn next(&mut self) -> Option<Requestor> {
        if self.count >= self.source.requestor_count {
            return None;
        }

        self.count += 1;

        let mut requestor = Requestor::new(
            self.source.max_price.sample(self.rng),
            self.source.budget_factor.sample(self.rng),
            self.defence_mechanism_type,
        );

        let count = self.source.subtask_count.sample(self.rng).round() as usize;
        let mut task = Task::new();

        for _ in 0..count {
            let nominal_usage = self.source.nominal_usage.sample(self.rng);
            let budget = requestor.max_price() * requestor.budget_factor() * nominal_usage;

            task.push_pending(SubTask::new(nominal_usage, budget));
        }

        requestor.task_queue_mut().push(task);

        Some(requestor)
    }
}

#[derive(Debug, Deserialize)]
pub struct ProviderSource {
    provider_count: usize,
    min_price: Generator,
    usage_factor: Generator,
    #[serde(default)]
    behaviour: ProviderBehaviour,
}

impl ProviderSource {
    pub fn iter<'a, Rng>(&'a self, rng: &'a mut Rng) -> ProviderSourceIter<Rng>
    where
        Rng: rand::Rng,
    {
        ProviderSourceIter {
            count: 0,
            source: self,
            rng,
        }
    }
}

#[derive(Debug)]
pub struct ProviderSourceIter<'a, Rng>
where
    Rng: rand::Rng,
{
    count: usize,
    source: &'a ProviderSource,
    rng: &'a mut Rng,
}

impl<'a, Rng> Iterator for ProviderSourceIter<'a, Rng>
where
    Rng: rand::Rng,
{
    type Item = Box<dyn Provider>;

    fn next(&mut self) -> Option<Box<dyn Provider>> {
        if self.count >= self.source.provider_count {
            return None;
        }

        self.count += 1;

        let min_price = self.source.min_price.sample(self.rng);
        let usage_factor = self.source.usage_factor.sample(self.rng);

        Some(match self.source.behaviour {
            ProviderBehaviour::UndercutBudget(epsilon) => Box::new(UndercutBudgetProvider::new(
                min_price,
                usage_factor,
                epsilon,
            )),
            ProviderBehaviour::LinearUsageInflation(factor) => Box::new(
                LinearUsageInflationProvider::new(min_price, usage_factor, factor),
            ),
            _ => Box::new(RegularProvider::new(min_price, usage_factor)),
        })
    }
}
