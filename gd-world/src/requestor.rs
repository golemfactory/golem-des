mod task_queue;

pub use self::task_queue::TaskQueue;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt;

use gd_engine::Engine;
use log::debug;
use num_traits::{NumAssign, NumCast};
use rand::distributions::Exp;
use rand::prelude::*;
use serde_derive::{Deserialize, Serialize};

use crate::error::SimulationError;
use crate::id::Id;
use crate::task::{SubTask, Task};
use crate::world::Event;

const REDUNDANCY_FACTOR: usize = 2;

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

#[allow(dead_code)]
#[derive(Debug)]
enum BanDuration<T>
where
    T: fmt::Debug + NumAssign + NumCast,
{
    Until(T),
    Indefinitely,
}

#[allow(dead_code)]
impl<T> BanDuration<T>
where
    T: fmt::Debug + NumAssign + NumCast,
{
    fn is_expired(&self) -> bool {
        match self {
            BanDuration::Until(value) => {
                *value == NumCast::from(0).expect("could not convert int to T")
            }
            BanDuration::Indefinitely => false,
        }
    }

    fn decrement(&mut self) {
        if let BanDuration::Until(ref mut value) = *self {
            *value -= NumCast::from(1).expect("could not convert int to T")
        }
    }
}

#[derive(Debug)]
pub struct Requestor {
    id: Id,
    max_price: f64,
    budget_factor: f64,
    task: Option<Task>,
    task_queue: TaskQueue,
    ratings: HashMap<Id, f64>,
    blacklist: HashMap<Id, BanDuration<i64>>,
    verification_map: HashMap<Id, Vec<Option<(Id, f64)>>>,
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

    pub fn new(max_price: f64, budget_factor: f64) -> Requestor {
        Requestor::with_id(Id::new(), max_price, budget_factor)
    }

    pub fn with_id(id: Id, max_price: f64, budget_factor: f64) -> Requestor {
        Requestor {
            id: id,
            max_price: max_price,
            budget_factor: budget_factor,
            task: None,
            task_queue: TaskQueue::new(),
            ratings: HashMap::new(),
            blacklist: HashMap::new(),
            verification_map: HashMap::new(),
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
        } else {
            if let Some(task) = self.task_queue.pop() {
                self.task = Some(task);
                self.num_tasks_advertised += 1;
                engine.schedule(
                    Exp::new(1.0 / Self::MEAN_TASK_ARRIVAL_TIME).sample(rng),
                    Event::TaskAdvertisement(self.id),
                );
            }
        }
    }

    pub fn receive_benchmark(
        &mut self,
        provider_id: &Id,
        reported_usage: f64,
    ) -> Result<(), SimulationError> {
        match self.ratings.insert(*provider_id, reported_usage) {
            Some(_) => Err(SimulationError::RatingAlreadyExists(self.id, *provider_id)),
            None => Ok(()),
        }
    }

    pub fn select_offers(
        &mut self,
        bids: Vec<(Id, f64)>,
    ) -> Result<Vec<(Id, SubTask, f64)>, SimulationError> {
        // filter offers
        let bids = self.filter_offers(bids);
        // rank offers by effective price
        let bids: Result<Vec<(Id, f64, f64)>, SimulationError> = bids
            .into_iter()
            .map(|(id, bid)| {
                let rating = self
                    .ratings
                    .get(&id)
                    .ok_or(SimulationError::RatingNotFound(self.id, id))?;
                Ok((id, bid, *rating))
            })
            .collect();
        let bids = self.rank_offers(bids?);

        // send available subtasks to eligible providers
        let mut messages: Vec<(Id, SubTask, f64)> = Vec::new();
        for chunk in bids.chunks_exact(REDUNDANCY_FACTOR) {
            match self
                .task
                .as_mut()
                .ok_or(SimulationError::TaskNotFound(self.id))?
                .pop_pending()
            {
                Some(subtask) => {
                    for &(provider_id, bid, _) in chunk {
                        debug!(
                            "R{}:sending {} to P{} for {}",
                            self.id, subtask, provider_id, bid
                        );

                        messages.push((provider_id, subtask, bid));
                    }

                    self.verification_map
                        .insert(*subtask.id(), Vec::with_capacity(REDUNDANCY_FACTOR));
                }
                None => break,
            }
        }

        Ok(messages)
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
            .filter(|(id, _)| !self.blacklist.contains_key(&id))
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

    fn rank_offers(&self, mut bids: Vec<(Id, f64, f64)>) -> Vec<(Id, f64, f64)> {
        bids.sort_unstable_by(|(_, x_bid, x_rating), (_, y_bid, y_rating)| {
            let x_price = x_bid * x_rating;
            let y_price = y_bid * y_rating;

            if x_price < y_price {
                Ordering::Less
            } else if x_price > y_price {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });

        bids.into_iter().collect()
    }

    pub fn verify_subtask(
        &mut self,
        subtask: &SubTask,
        provider_id: &Id,
        _bid: f64,
        reported_usage: f64,
    ) -> Result<(), SimulationError> {
        debug!("R{}:verifying {}", self.id, subtask);

        let rating = self
            .ratings
            .get(&provider_id)
            .ok_or(SimulationError::RatingNotFound(self.id, *provider_id))?;
        self.verification_map
            .get_mut(subtask.id())
            .ok_or(SimulationError::VerificationForSubtaskNotFound(
                self.id, *subtask,
            ))?
            .push(Some((*provider_id, reported_usage / rating)));

        if self.verification_map.get(subtask.id()).unwrap().len() < REDUNDANCY_FACTOR {
            return Ok(());
        }

        let vers: Vec<(Id, f64)> = self
            .verification_map
            .remove(subtask.id())
            .ok_or(SimulationError::VerificationForSubtaskNotFound(
                self.id, *subtask,
            ))?
            .into_iter()
            .filter_map(|v| v)
            .collect();

        if vers.len() == 2 {
            if vers[0].1 > vers[1].1 {
                self.update_rating(vers[0], vers[1])?;
            } else {
                self.update_rating(vers[1], vers[0])?;
            }
        }

        self.num_subtasks_computed += 1;
        self.task
            .as_mut()
            .ok_or(SimulationError::TaskNotFound(self.id))?
            .push_done(*subtask);

        Ok(())
    }

    fn update_rating(&mut self, p1: (Id, f64), p2: (Id, f64)) -> Result<(), SimulationError> {
        let (id1, d1) = p1;
        let (_, d2) = p2;
        let usage_factor = self
            .ratings
            .get_mut(&id1)
            .ok_or(SimulationError::RatingNotFound(self.id, id1))?;
        let old_rating = *usage_factor;
        *usage_factor *= d1 / d2;

        debug!(
            "R{}:P{} failed verification, rating {} => {}",
            self.id, id1, old_rating, *usage_factor
        );

        if *usage_factor >= 2.0 {
            let duration = BanDuration::Indefinitely;

            debug!("R{}:P{} blacklisted for {:?}", self.id, id1, duration);

            self.blacklist.insert(id1, duration);
        }

        Ok(())
    }

    pub fn send_payment(
        &mut self,
        subtask: &SubTask,
        provider_id: &Id,
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

    pub fn complete_task(&mut self) -> Result<(), SimulationError> {
        if self
            .task
            .as_ref()
            .ok_or(SimulationError::TaskNotFound(self.id))?
            .is_done()
        {
            debug!("R{}:task computed", self.id);

            self.num_tasks_computed += 1;
            self.task = None;
        }

        Ok(())
    }

    pub fn budget_exceeded(
        &mut self,
        subtask: &SubTask,
        provider_id: &Id,
    ) -> Result<(), SimulationError> {
        debug!(
            "R{}:budget exceeded for {} by P{}",
            self.id, subtask, provider_id
        );

        self.verification_map
            .get_mut(subtask.id())
            .ok_or(SimulationError::VerificationForSubtaskNotFound(
                self.id, *subtask,
            ))?
            .push(None);

        if self
            .verification_map
            .get(subtask.id())
            .ok_or(SimulationError::VerificationForSubtaskNotFound(
                self.id, *subtask,
            ))?
            .len()
            < REDUNDANCY_FACTOR
        {
            return Ok(());
        }

        let vers: Vec<(Id, f64)> = self
            .verification_map
            .remove(subtask.id())
            .ok_or(SimulationError::VerificationForSubtaskNotFound(
                self.id, *subtask,
            ))?
            .into_iter()
            .filter_map(|v| v)
            .collect();

        if vers.len() > 0 {
            self.num_subtasks_computed += 1;
            self.task
                .as_mut()
                .ok_or(SimulationError::TaskNotFound(self.id))?
                .push_done(*subtask);
        } else {
            self.num_subtasks_cancelled += 1;
            self.task
                .as_mut()
                .ok_or(SimulationError::TaskNotFound(self.id))?
                .push_pending(*subtask);
        }

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receive_benchmark() {
        let mut requestor = Requestor::new(1.0, 1.0);
        let p1 = (Id::new(), 0.5);
        let p2 = (Id::new(), 0.75);

        assert_eq!(requestor.receive_benchmark(&p1.0, p1.1).ok(), Some(()));
        assert_eq!(requestor.receive_benchmark(&p2.0, p2.1).ok(), Some(()));
        assert_eq!(
            requestor.receive_benchmark(&p1.0, p1.1).err(),
            Some(SimulationError::RatingAlreadyExists(requestor.id, p1.0))
        );
    }

    #[test]
    fn test_rank_offers() {
        let requestor = Requestor::new(1.0, 1.0);
        let bid1 = (Id::new(), 2.5, 0.25); // (provider_id, bid/offer, rating stored at the requestor)
        let bid2 = (Id::new(), 0.5, 0.75);

        assert_eq!(requestor.rank_offers(vec![bid1, bid2]), vec![bid2, bid1]);
    }

    #[test]
    fn test_filter_offers() {
        let mut requestor = Requestor::new(1.0, 1.0);
        let bid1 = (Id::new(), 1.0); // (provider_id, bid/offer)
        let bid2 = (Id::new(), 2.0);

        assert!(requestor.blacklist.is_empty());
        assert_eq!(requestor.filter_offers(vec![bid1, bid2]), vec![bid1, bid2]);

        requestor
            .blacklist
            .insert(bid1.0, BanDuration::Indefinitely);

        assert!(!requestor.blacklist.is_empty());
        assert_eq!(requestor.filter_offers(vec![bid1, bid2]), vec![bid2]);
    }

    #[test]
    fn test_select_offers() {
        let mut requestor = Requestor::new(1.0, 1.0);
        let subtask = SubTask::new(1.0, 1.0);
        let mut task = Task::new();
        task.push_pending(subtask);
        requestor.task_queue.push(task.clone());

        assert_eq!(requestor.task, None);

        requestor.task = requestor.task_queue.pop();

        assert_eq!(requestor.task, Some(task));

        let bid1 = (Id::new(), 1.0);
        let bid2 = (Id::new(), 2.0);

        requestor.ratings.insert(bid1.0, 1.0);
        requestor.ratings.insert(bid2.0, 1.0);

        let messages = requestor.select_offers(vec![bid1, bid2]).unwrap();

        assert_eq!(
            messages,
            vec![(bid1.0, subtask, 1.0), (bid2.0, subtask, 2.0)]
        );
    }
}
