mod defence;

pub use self::defence::{CTasks, DefenceMechanism, DefenceMechanismType, LGRola, Redundancy};

use std::cmp::Ordering;
use std::collections::VecDeque;
use std::fmt;

use gd_engine::Engine;
use log::debug;
use rand::distributions::Exp;
use rand::prelude::*;
use serde_derive::{Deserialize, Serialize};

use crate::id::Id;
use crate::task::{SubTask, Task};
use crate::world::Event;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub run_id: u64,
    pub max_price: f64,
    pub budget_factor: f64,
    pub mean_cost: f64,
    pub num_tasks_advertised: usize,
    pub num_tasks_computed: usize,
    pub num_readvertisements: usize,
    pub num_subtasks_computed: usize,
    pub num_subtasks_cancelled: usize,
}

#[derive(Debug)]
struct TaskQueue {
    buffer: VecDeque<Task>,
    repeating: bool,
}

impl TaskQueue {
    fn new(repeating: bool) -> TaskQueue {
        TaskQueue {
            buffer: VecDeque::new(),
            repeating: repeating,
        }
    }

    fn pop(&mut self) -> Option<Task> {
        self.buffer.pop_front().map(|task| {
            if self.repeating {
                self.buffer.push_back(task.replicate());
            }

            task
        })
    }
}

#[derive(Debug)]
pub struct Requestor {
    id: Id,
    max_price: f64,
    budget_factor: f64,
    task: Option<Task>,
    task_queue: TaskQueue,
    defence_mechanism: Box<dyn DefenceMechanism>,
    mean_cost: (usize, f64),
    num_tasks_advertised: usize,
    num_tasks_computed: usize,
    num_readvertisements: usize,
    num_subtasks_computed: usize,
    num_subtasks_cancelled: usize,
}

impl Requestor {
    const MEAN_TASK_ARRIVAL_TIME: f64 = 900.0;
    const READVERT_DELAY: f64 = 60.0;

    pub fn new(
        max_price: f64,
        budget_factor: f64,
        repeating: bool,
        defence_mechanism_type: DefenceMechanismType,
    ) -> Requestor {
        Requestor::with_id(
            Id::new(),
            max_price,
            budget_factor,
            repeating,
            defence_mechanism_type,
        )
    }

    pub fn with_id(
        id: Id,
        max_price: f64,
        budget_factor: f64,
        repeating: bool,
        defence_mechanism_type: DefenceMechanismType,
    ) -> Requestor {
        Requestor {
            id: id,
            max_price: max_price,
            budget_factor: budget_factor,
            task: None,
            task_queue: TaskQueue::new(repeating),
            defence_mechanism: defence_mechanism_type.into_dm(id),
            mean_cost: (0, 0.0),
            num_tasks_advertised: 0,
            num_tasks_computed: 0,
            num_readvertisements: 0,
            num_subtasks_computed: 0,
            num_subtasks_cancelled: 0,
        }
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn max_price(&self) -> f64 {
        self.max_price
    }

    pub fn budget_factor(&self) -> f64 {
        self.budget_factor
    }

    pub fn push_task(&mut self, task: Task) {
        self.task_queue.buffer.push_back(task)
    }

    pub fn append_tasks<It: IntoIterator<Item = Task>>(&mut self, tasks: It) {
        for task in tasks {
            self.push_task(task);
        }
    }

    pub fn advertise<Rng>(&mut self, engine: &mut Engine<Event>, rng: &mut Rng)
    where
        Rng: rand::Rng,
    {
        if let Some(task) = &self.task {
            if !task.is_done() {
                panic!(
                    "R{}:cannot advertise new task as {} already pending",
                    self.id, task
                );
            }
        }

        if let Some(task) = self.task_queue.pop() {
            self.task = Some(task);
            self.num_tasks_advertised += 1;

            engine.schedule(
                Exp::new(1.0 / Self::MEAN_TASK_ARRIVAL_TIME).sample(rng),
                Event::TaskAdvertisement(self.id),
            );
        }
    }

    pub fn receive_benchmark(&mut self, provider_id: &Id, reported_usage: f64) {
        self.defence_mechanism
            .insert_provider_rating(provider_id, reported_usage);
    }

    pub fn select_offers(&mut self, bids: Vec<(Id, f64)>) -> Vec<(Id, SubTask, f64)> {
        let mut bids = self.filter_offers(bids);
        self.rank_offers(&mut bids);
        self.defence_mechanism.schedule_subtasks(
            self.task
                .as_mut()
                .expect(&format!("R{}:task not found", self.id)),
            bids,
        )
    }

    fn filter_offers(&mut self, bids: Vec<(Id, f64)>) -> Vec<(Id, f64)> {
        debug!(
            "R{}:offers before filtering = {{ {} }}",
            self.id,
            bids.iter()
                .map(|(id, bid)| format!("{} => {}", id, bid))
                .fold(String::new(), |acc, s| acc + &s + ", ")
        );

        let bids: Vec<(Id, f64)> = bids
            .into_iter()
            .filter(|(id, _)| !self.defence_mechanism.is_blacklisted(&id))
            .collect();

        debug!(
            "R{}:offers after filtering = {{ {} }}",
            self.id,
            bids.iter()
                .map(|(id, bid)| format!("{} => {}", id, bid))
                .fold(String::new(), |acc, s| acc + &s + ", ")
        );

        bids
    }

    fn rank_offers(&self, bids: &mut Vec<(Id, f64)>) {
        bids.sort_unstable_by(|(x_id, x_bid), (y_id, y_bid)| {
            let x_usage = self.defence_mechanism.get_rating(x_id).expect(&format!(
                "R{}: usage rating for P{} not found",
                self.id, x_id
            ));
            let y_usage = self.defence_mechanism.get_rating(y_id).expect(&format!(
                "R{}: usage rating for P{} not found",
                self.id, y_id
            ));

            let x_price = x_bid * x_usage;
            let y_price = y_bid * y_usage;

            if x_price < y_price {
                Ordering::Less
            } else if x_price > y_price {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });
    }

    pub fn verify_subtask(
        &mut self,
        provider_id: Id,
        subtask: SubTask,
        bid: f64,
        reported_usage: f64,
    ) {
        debug!("R{}:{} computed by P{}", self.id, subtask, provider_id);

        let payment = reported_usage * bid;

        debug!("R{}:for {}, incurred cost {}", self.id, subtask, payment);

        self.defence_mechanism
            .subtask_computed(&subtask, &provider_id, reported_usage, bid);

        let (count, current_mean) = &mut self.mean_cost;
        let current_cost_wrt_budget = bid * reported_usage / subtask.budget;
        *count += 1;
        *current_mean = *current_mean + (current_cost_wrt_budget - *current_mean) / (*count as f64);

        debug!(
            "R{}:current cost wrt budget = {}%, average cost wrt budget = {}%",
            self.id,
            current_cost_wrt_budget * 100.0,
            *current_mean * 100.0,
        );

        self.num_subtasks_computed += 1;
        self.task
            .as_mut()
            .expect(&format!("R{}:task not found!", self.id))
            .subtask_computed(subtask);
    }

    pub fn send_payment(
        &mut self,
        provider_id: Id,
        subtask: SubTask,
        bid: f64,
        reported_usage: f64,
    ) -> Option<f64> {
        debug!(
            "R{}:sending payment {} for {} to P{}",
            self.id,
            reported_usage * bid,
            subtask,
            provider_id
        );

        Some(reported_usage * bid)
    }

    pub fn complete_task<Rng>(&mut self, engine: &mut Engine<Event>, rng: &mut Rng)
    where
        Rng: rand::Rng,
    {
        if self
            .task
            .as_ref()
            .expect(&format!("R{}:task not found", self.id))
            .is_done()
        {
            debug!("R{}:task computed", self.id);

            self.defence_mechanism.task_computed();
            self.num_tasks_computed += 1;
            self.advertise(engine, rng);
        } //else if self
          // .task
          // .as_ref()
          // .expect(&format!("R{}:task not found", self.id))
          // .is_pending()
          // {
          // self.num_readvertisements += 1;
          // engine.schedule(Self::READVERT_DELAY, Event::TaskAdvertisement(self.id));
          // }
    }

    pub fn readvertise(&mut self, engine: &mut Engine<Event>) {
        if self
            .task
            .as_ref()
            .expect(&format!("R{}:task not found", self.id))
            .is_pending()
        {
            self.num_readvertisements += 1;
            engine.schedule(Self::READVERT_DELAY, Event::TaskAdvertisement(self.id));
        }
    }

    pub fn budget_exceeded(
        &mut self,
        engine: &mut Engine<Event>,
        provider_id: Id,
        subtask: SubTask,
    ) {
        debug!(
            "R{}:budget exceeded for {} by P{}",
            self.id, subtask, provider_id
        );

        self.num_subtasks_cancelled += 1;
        self.num_readvertisements += 1;

        self.task
            .as_mut()
            .expect(&format!("R{}:task not found!", self.id))
            .push_subtask(subtask);

        engine.schedule(Self::READVERT_DELAY, Event::TaskAdvertisement(self.id));
    }

    pub fn into_stats(self, run_id: u64) -> Stats {
        Stats {
            run_id: run_id,
            max_price: self.max_price,
            budget_factor: self.budget_factor,
            mean_cost: self.mean_cost.1 * 100.0,
            num_tasks_advertised: self.num_tasks_advertised,
            num_tasks_computed: self.num_tasks_computed,
            num_readvertisements: self.num_readvertisements,
            num_subtasks_computed: self.num_subtasks_computed,
            num_subtasks_cancelled: self.num_subtasks_cancelled,
        }
    }
}

impl fmt::Display for Requestor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            r"Requestor
            Id:                             {},
            Max price:                      {},
            Budget factor:                  {},
            Mean cost wrt budget:           {},
            Number of tasks advertised:     {},
            Number of tasks computed:       {},
            Number of readvertisements:     {},
            Number of subtasks computed:    {},
            Number of subtasks cancelled:   {},
            ",
            self.id,
            self.max_price,
            self.budget_factor,
            self.mean_cost.1 * 100.0,
            self.num_tasks_advertised,
            self.num_tasks_computed,
            self.num_readvertisements,
            self.num_subtasks_computed,
            self.num_subtasks_cancelled,
        )
    }
}
