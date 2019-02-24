mod defence;
mod task_queue;

pub use self::defence::{DefenceMechanism, DefenceMechanismType, Redundancy};
pub use self::task_queue::TaskQueue;

use std::fmt;

use gd_engine::Engine;
use log::debug;
use rand::distributions::Exp;
use rand::prelude::*;
use serde_derive::{Deserialize, Serialize};

use crate::id::Id;
use crate::task::subtask;
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
    const MEAN_TASK_ARRIVAL_TIME: f64 = 3600.0;
    const READVERT_DELAY: f64 = 60.0;

    pub fn new(max_price: f64, budget_factor: f64, dm_type: DefenceMechanismType) -> Self {
        Self::with_id(Id::new(), max_price, budget_factor, dm_type)
    }

    pub fn with_id(
        id: Id,
        max_price: f64,
        budget_factor: f64,
        dm_type: DefenceMechanismType,
    ) -> Self {
        Self {
            id,
            max_price,
            budget_factor,
            task: None,
            task_queue: TaskQueue::new(),
            defence_mechanism: dm_type.into_dm(id),
            mean_cost: (0, 0.0),
            num_tasks_advertised: 0,
            num_tasks_computed: 0,
            num_readvertisements: 0,
            num_subtasks_computed: 0,
            num_subtasks_cancelled: 0,
        }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn max_price(&self) -> f64 {
        self.max_price
    }

    pub fn budget_factor(&self) -> f64 {
        self.budget_factor
    }

    pub fn task_queue(&self) -> &TaskQueue {
        &self.task_queue
    }

    pub fn task_queue_mut(&mut self) -> &mut TaskQueue {
        &mut self.task_queue
    }

    pub fn advertise<Rng>(&mut self, engine: &mut Engine<Event>, rng: &mut Rng)
    where
        Rng: rand::Rng,
    {
        if let Some(task) = &self.task {
            if task.is_pending() {
                self.num_readvertisements += 1;
                engine.schedule(Self::READVERT_DELAY, Event::TaskAdvertisement(self.id));
            }
        } else if let Some(task) = self.task_queue.pop() {
            self.task = Some(task);
            self.num_tasks_advertised += 1;
            engine.schedule(
                Exp::new(1.0 / Self::MEAN_TASK_ARRIVAL_TIME).sample(rng),
                Event::TaskAdvertisement(self.id),
            );
        }
    }

    pub fn receive_benchmark(&mut self, provider_id: Id, reported_usage: f64) {
        self.defence_mechanism
            .insert_provider_rating(provider_id, reported_usage)
    }

    pub fn select_offers(&mut self, bids: Vec<(Id, f64)>) -> Vec<(Id, SubTask, f64)> {
        // send available subtasks to eligible providers
        let task = self.task.as_mut().expect("task not found");

        self.defence_mechanism.assign_subtasks(task, bids)
    }

    pub fn verify_subtask(
        &mut self,
        subtask: &SubTask,
        provider_id: Id,
        reported_usage: Option<f64>,
    ) {
        debug!("R{}:verifying {}", self.id, subtask);

        match self
            .defence_mechanism
            .verify_subtask(subtask, provider_id, reported_usage)
        {
            subtask::Status::Done => {
                self.num_subtasks_computed += 1;
                self.task
                    .as_mut()
                    .expect("task not found")
                    .push_done(*subtask);
            }
            subtask::Status::Cancelled => {
                self.num_subtasks_cancelled += 1;
                self.task
                    .as_mut()
                    .expect("task not found")
                    .push_pending(*subtask);
            }
            subtask::Status::Pending => {}
        }
    }

    pub fn send_payment(
        &mut self,
        subtask: &SubTask,
        provider_id: Id,
        bid: f64,
        reported_usage: f64,
    ) -> Option<f64> {
        debug!("R{}:{} computed by P{}", self.id, subtask, provider_id);

        let payment = reported_usage * bid;

        debug!("R{}:for {}, incurred cost {}", self.id, subtask, payment);

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
        debug!(
            "R{}:sending payment {} for {} to P{}",
            self.id, payment, subtask, provider_id
        );

        Some(payment)
    }

    pub fn complete_task(&mut self) {
        if self.task.as_ref().expect("task not found").is_done() {
            debug!("R{}:task computed", self.id);

            self.defence_mechanism.complete_task();
            self.num_tasks_computed += 1;
            self.task = None;
        }
    }

    pub fn into_stats(self, run_id: u64) -> Stats {
        Stats {
            run_id,
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

#[cfg(test)]
mod tests {
    use super::*;

    use statrs::assert_almost_eq;

    #[test]
    fn send_payment() {
        let mut requestor = Requestor::new(1.0, 1.0, DefenceMechanismType::Redundancy);
        let p1 = (SubTask::new(100.0, 100.0), Id::new(), 0.1, 50.0); // (subtask, provider_id, bid, usage)

        assert_eq!(requestor.send_payment(&p1.0, p1.1, p1.2, p1.3), Some(5.0));

        assert_eq!(requestor.mean_cost.0, 1);
        assert_almost_eq!(requestor.mean_cost.1, 0.05, 1e-5);
    }

    #[test]
    fn complete_task() {
        let mut requestor = Requestor::new(1.0, 1.0, DefenceMechanismType::Redundancy);
        let task = Task::new();
        requestor.task_queue.push(task.clone());
        requestor.task = requestor.task_queue.pop();

        assert_eq!(requestor.task, Some(task));
        assert!(requestor.task.as_ref().unwrap().is_done());

        requestor.complete_task();

        assert_eq!(requestor.task, None);
    }
}
