use std::collections::{HashMap, VecDeque};

use log::debug;
use statrs::statistics::{OrderStatistics, Statistics};

use super::{BanDuration, DefenceMechanism, DefenceMechanismCommon};

use crate::id::Id;
use crate::task::{SubTask, Task};

#[derive(Debug)]
pub struct LGRola {
    common: DefenceMechanismCommon,
    task_usages: HashMap<Id, Vec<f64>>,
    collisions: HashMap<Id, u32>,
}

impl LGRola {
    pub fn new(id: Id) -> LGRola {
        LGRola {
            common: DefenceMechanismCommon::new(id),
            task_usages: HashMap::new(),
            collisions: HashMap::new(),
        }
    }
}

impl DefenceMechanism for LGRola {
    fn schedule_subtasks(
        &mut self,
        subtask_queue: &mut VecDeque<Task>,
        bids: Vec<(Id, f64)>,
    ) -> Vec<(Id, SubTask, f64)> {
        let self_id = self.as_dm_common().id;
        let mut messages: Vec<(Id, SubTask, f64)> = Vec::new();

        for (provider_id, bid) in bids {
            match subtask_queue
                .front_mut()
                .expect(&format!("R{}:task not found!", self_id))
                .pop_subtask()
            {
                Some(subtask) => {
                    debug!(
                        "R{}:sending {} to P{} for {}",
                        self_id, subtask, provider_id, bid
                    );

                    messages.push((provider_id, subtask, bid));
                }
                None => break,
            }
        }

        messages
    }

    fn subtask_computed(
        &mut self,
        _subtask: &SubTask,
        provider_id: &Id,
        reported_usage: f64,
        _bid: f64,
    ) {
        self.task_usages
            .entry(*provider_id)
            .or_insert(Vec::new())
            .push(reported_usage);
    }

    fn task_computed(&mut self) {
        let ids_to_remove: Vec<Id> = self
            .as_dm_common()
            .blacklist
            .iter()
            .filter_map(|(&id, until)| if until.is_expired() { Some(id) } else { None })
            .collect();

        for id in ids_to_remove {
            self.as_dm_common_mut().blacklist.remove(&id);
        }

        for (_, until) in &mut self.as_dm_common_mut().blacklist {
            until.decrement();
        }

        let task_usages: Vec<(f64, Id)> = self
            .task_usages
            .iter()
            .map(|(&id, usages)| (usages.iter().geometric_mean(), id))
            .collect();

        let mut only_usages: Vec<f64> = task_usages.iter().map(|&(usage, _)| usage).collect();
        let q3 = only_usages.upper_quartile();
        let iqr = only_usages.interquartile_range();
        let threshold = 1.5 * iqr + q3;

        let self_id = self.as_dm_common().id;

        for &(usage, id) in &task_usages {
            if usage > threshold {
                let collisions = self
                    .collisions
                    .entry(id)
                    .and_modify(|collisions| *collisions += 1)
                    .or_insert(1);

                let until = BanDuration::Until((*collisions as f64).exp().ceil() as i64);

                debug!(
                    "R{}:P{} blacklisted for {:?} requests, usage = {}, collisions = {}",
                    self_id, id, until, usage, *collisions
                );

                self.as_dm_common_mut().blacklist.insert(id, until);
            } else {
                self.collisions
                    .entry(id)
                    .and_modify(|collisions| {
                        if *collisions > 0 {
                            *collisions -= 1;
                        }
                    })
                    .or_insert(0);
            }
        }

        debug!(
            "R{}:collisions map = {{ {} }}",
            self_id,
            self.collisions
                .iter()
                .map(|(id, penalty)| format!("{} => {}", id, penalty))
                .fold(String::new(), |acc, s| acc + &s + ", ")
        );

        debug!(
            "R{}:blacklisted map = {{ {} }}",
            self_id,
            self.as_dm_common()
                .blacklist
                .iter()
                .map(|(id, timeout)| format!("{} => {:?}", id, timeout))
                .fold(String::new(), |acc, s| acc + &s + ", ")
        );

        self.task_usages.clear();
    }

    fn as_dm_common(&self) -> &DefenceMechanismCommon {
        &self.common
    }

    fn as_dm_common_mut(&mut self) -> &mut DefenceMechanismCommon {
        &mut self.common
    }
}
