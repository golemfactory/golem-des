mod task_queue;
mod verification;

pub use self::task_queue::TaskQueue;
pub use self::verification::VerificationMap;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt;

use gd_engine::Engine;
use log::{debug, warn};
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
pub struct Requestor {
    id: Id,
    max_price: f64,
    budget_factor: f64,
    task: Option<Task>,
    task_queue: TaskQueue,
    ratings: HashMap<Id, f64>,
    blacklisted_set: HashSet<Id>,
    verification_map: VerificationMap<Id>,
    mean_cost: (usize, f64),
    num_tasks_advertised: usize,
    num_tasks_computed: usize,
    num_readvertisements: usize,
    num_subtasks_computed: usize,
    num_subtasks_cancelled: usize,
}

impl Requestor {
    const REDUNDANCY_FACTOR: usize = 2;
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
            blacklisted_set: HashSet::new(),
            verification_map: VerificationMap::new(Self::REDUNDANCY_FACTOR),
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

    pub fn receive_benchmark(&mut self, provider_id: &Id, reported_usage: f64) {
        if let Some(old_rating) = self.ratings.insert(*provider_id, reported_usage) {
            warn!(
                "R{}:rating for P{} already existed, replacing: {} => {}",
                self.id, provider_id, old_rating, reported_usage
            )
        }
    }

    pub fn select_offers(&mut self, bids: Vec<(Id, f64)>) -> Vec<(Id, SubTask, f64)> {
        // filter offers
        let bids = self.filter_offers(bids);
        // rank offers by effective price
        let bids = self.rank_offers(bids);
        // send available subtasks to eligible providers
        let mut messages: Vec<(Id, SubTask, f64)> = Vec::new();
        for chunk in bids.chunks_exact(Self::REDUNDANCY_FACTOR) {
            match self.task.as_mut().expect("task not found").pop_pending() {
                Some(subtask) => {
                    for &(provider_id, bid) in chunk {
                        debug!(
                            "R{}:sending {} to P{} for {}",
                            self.id, subtask, provider_id, bid
                        );

                        messages.push((provider_id, subtask, bid));
                    }

                    self.verification_map.insert_key(*subtask.id());
                }
                None => break,
            }
        }

        messages
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
            .filter(|(id, _)| !self.blacklisted_set.contains(&id))
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

    fn rank_offers(&self, mut bids: Vec<(Id, f64)>) -> Vec<(Id, f64)> {
        bids.sort_unstable_by(|(x_id, x_bid), (y_id, y_bid)| {
            let x_rating = self.ratings.get(&x_id).expect("rating not found");
            let y_rating = self.ratings.get(&y_id).expect("rating not found");

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
        reported_usage: Option<f64>,
    ) {
        debug!("R{}:verifying {}", self.id, subtask);

        let ver_res = reported_usage.map(|usage| {
            let rating = self.ratings.get(&provider_id).expect("rating not found");
            (*provider_id, usage / rating)
        });
        if let Some(vers) = self
            .verification_map
            .insert_verification(subtask.id(), ver_res)
        {
            if vers.len() > 0 {
                if vers.len() == Self::REDUNDANCY_FACTOR {
                    if vers[0].1 > vers[1].1 {
                        self.update_rating(vers[0], vers[1]);
                    } else {
                        self.update_rating(vers[1], vers[0]);
                    }
                }

                self.num_subtasks_computed += 1;
                self.task
                    .as_mut()
                    .expect("task not found")
                    .push_done(*subtask);
            } else {
                self.num_subtasks_cancelled += 1;
                self.task
                    .as_mut()
                    .expect("task not found")
                    .push_pending(*subtask);
            }
        }
    }

    fn update_rating(&mut self, p1: (Id, f64), p2: (Id, f64)) {
        let (id1, d1) = p1;
        let (_, d2) = p2;
        let usage_factor = self.ratings.get_mut(&id1).expect("rating not found");
        let old_rating = *usage_factor;
        *usage_factor *= d1 / d2;

        debug!(
            "R{}:P{} failed verification, rating {} => {}",
            self.id, id1, old_rating, *usage_factor
        );

        if *usage_factor >= 2.0 {
            debug!("R{}:P{} blacklisted", self.id, id1);

            self.blacklisted_set.insert(id1);
        }
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

    pub fn complete_task(&mut self) {
        if self.task.as_ref().expect("task not found").is_done() {
            debug!("R{}:task computed", self.id);

            self.num_tasks_computed += 1;
            self.task = None;
        }
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

    use statrs::assert_almost_eq;

    #[test]
    fn test_rank_offers() {
        let mut requestor = Requestor::new(1.0, 1.0);
        let bid1 = (Id::new(), 2.5); // (provider_id, bid/offer)
        let bid2 = (Id::new(), 0.5);
        requestor.ratings.insert(bid1.0, 0.25);
        requestor.ratings.insert(bid2.0, 0.75);

        assert_eq!(requestor.rank_offers(vec![bid1, bid2]), vec![bid2, bid1]);
    }

    #[test]
    fn test_filter_offers() {
        let mut requestor = Requestor::new(1.0, 1.0);
        let bid1 = (Id::new(), 1.0); // (provider_id, bid/offer)
        let bid2 = (Id::new(), 2.0);

        assert!(requestor.blacklisted_set.is_empty());
        assert_eq!(requestor.filter_offers(vec![bid1, bid2]), vec![bid1, bid2]);

        requestor.blacklisted_set.insert(bid1.0);

        assert!(!requestor.blacklisted_set.is_empty());
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

        assert_eq!(
            requestor.select_offers(vec![bid1, bid2]),
            vec![(bid1.0, subtask, 1.0), (bid2.0, subtask, 2.0)]
        );
    }

    #[test]
    fn test_update_rating() {
        let mut requestor = Requestor::new(1.0, 1.0);
        let p1 = (Id::new(), 0.25, 25.0); // (provider_id, rating, usage)
        let p2 = (Id::new(), 0.75, 75.0);
        requestor.ratings.insert(p1.0, p1.1);
        requestor.ratings.insert(p2.0, p2.1);

        requestor.update_rating((p1.0, p1.2 / p1.1), (p2.0, p2.2 / p2.1));

        assert_almost_eq!(*requestor.ratings.get(&p1.0).unwrap(), 0.25, 1e-5);
        assert_almost_eq!(*requestor.ratings.get(&p2.0).unwrap(), 0.75, 1e-5);
        assert!(requestor.blacklisted_set.is_empty());

        requestor.update_rating((p1.0, 200.0), (p2.0, p2.2 / p2.1));

        assert_almost_eq!(*requestor.ratings.get(&p1.0).unwrap(), 0.5, 1e-5);
        assert_almost_eq!(*requestor.ratings.get(&p2.0).unwrap(), 0.75, 1e-5);
        assert!(requestor.blacklisted_set.is_empty());

        requestor.update_rating((p1.0, 400.0), (p2.0, p2.2 / p2.1));

        assert_almost_eq!(*requestor.ratings.get(&p1.0).unwrap(), 2.0, 1e-5);
        assert_almost_eq!(*requestor.ratings.get(&p2.0).unwrap(), 0.75, 1e-5);
        assert!(requestor.blacklisted_set.contains(&p1.0));
    }

    #[test]
    fn test_send_payment() {
        let mut requestor = Requestor::new(1.0, 1.0);
        let p1 = (SubTask::new(100.0, 100.0), Id::new(), 0.1, 50.0); // (subtask, provider_id, bid, usage)

        assert_eq!(requestor.send_payment(&p1.0, &p1.1, p1.2, p1.3), Some(5.0));

        assert_eq!(requestor.mean_cost.0, 1);
        assert_almost_eq!(requestor.mean_cost.1, 0.05, 1e-5);
    }

    #[test]
    fn test_complete_task() {
        let mut requestor = Requestor::new(1.0, 1.0);
        let task = Task::new();
        requestor.task_queue.push(task.clone());
        requestor.task = requestor.task_queue.pop();

        assert_eq!(requestor.task, Some(task));
        assert!(requestor.task.as_ref().unwrap().is_done());

        requestor.complete_task();

        assert_eq!(requestor.task, None);
    }

    #[test]
    fn test_all_verification_paths() {
        let setup = || {
            let mut requestor = Requestor::new(1.0, 1.0);
            let mut task = Task::new();
            task.push_pending(SubTask::new(100.0, 100.0));
            requestor.task_queue.push(task);
            requestor.task = requestor.task_queue.pop();

            let p1 = (Id::new(), 0.25, 100.0);
            let p2 = (Id::new(), 0.75, 75.0);
            requestor.ratings.insert(p1.0, p1.1);
            requestor.ratings.insert(p2.0, p2.1);

            let subtask = requestor.task.as_mut().unwrap().pop_pending().unwrap();
            requestor.verification_map.insert_key(*subtask.id());

            assert!(!requestor.task.as_ref().unwrap().is_pending());
            assert!(!requestor.task.as_ref().unwrap().is_done());

            (requestor, subtask, p1, p2)
        };

        // 1. (success, success)
        {
            let (mut requestor, subtask, p1, p2) = setup();

            requestor.verify_subtask(&subtask, &p1.0, Some(p1.2));
            requestor.verify_subtask(&subtask, &p2.0, Some(p2.2));

            assert!(!requestor.task.as_ref().unwrap().is_pending());
            assert!(requestor.task.as_ref().unwrap().is_done());
        }

        // 2. (success, budget_exceeded)
        {
            let (mut requestor, subtask, p1, p2) = setup();

            requestor.verify_subtask(&subtask, &p1.0, Some(p1.2));
            requestor.verify_subtask(&subtask, &p2.0, None);

            assert!(!requestor.task.as_ref().unwrap().is_pending());
            assert!(requestor.task.as_ref().unwrap().is_done());
        }

        // 3. (budget_exceeded, success)
        {
            let (mut requestor, subtask, p1, p2) = setup();

            requestor.verify_subtask(&subtask, &p1.0, None);
            requestor.verify_subtask(&subtask, &p2.0, Some(p2.2));

            assert!(!requestor.task.as_ref().unwrap().is_pending());
            assert!(requestor.task.as_ref().unwrap().is_done());
        }
        
        // 4. (budget_exceeded, budget_exceeded)
        {
            let (mut requestor, subtask, p1, p2) = setup();

            requestor.verify_subtask(&subtask, &p1.0, None);
            requestor.verify_subtask(&subtask, &p2.0, None);

            assert!(requestor.task.as_ref().unwrap().is_pending());
            assert!(!requestor.task.as_ref().unwrap().is_done());
        }
    }
}
