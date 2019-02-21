use std::collections::HashMap;

use gd_world_derive::DerefDefenceMechanismCommon;
use log::debug;
use statrs::statistics::{OrderStatistics, Statistics};

use super::{BanDuration, DefenceMechanism, DefenceMechanismCommon};

use crate::id::Id;
use crate::task::subtask;
use crate::task::{SubTask, Task};

#[derive(Debug, DerefDefenceMechanismCommon)]
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
    fn assign_subtasks(
        &mut self,
        task: &mut Task,
        bids: Vec<(Id, f64)>,
    ) -> Vec<(Id, SubTask, f64)> {
        let bids = self.filter_offers(bids);
        let bids = self.rank_offers(bids);

        let mut messages: Vec<(Id, SubTask, f64)> = Vec::new();

        for (provider_id, bid) in bids {
            match task.pop_pending() {
                Some(subtask) => {
                    debug!(
                        "R{}:sending {} to P{} for {}",
                        self.requestor_id, subtask, provider_id, bid
                    );

                    messages.push((provider_id, subtask, bid));
                }
                None => break,
            }
        }

        messages
    }

    fn verify_subtask(
        &mut self,
        _subtask: &SubTask,
        provider_id: &Id,
        reported_usage: Option<f64>,
    ) -> subtask::Status {
        reported_usage.map_or(subtask::Status::Cancelled, |usage| {
            self.task_usages
                .entry(*provider_id)
                .or_insert(Vec::new())
                .push(usage);

            subtask::Status::Done
        })
    }

    fn complete_task(&mut self) {
        let ids_to_remove: Vec<Id> = self
            .blacklisted_set
            .iter()
            .filter_map(|(&id, until)| if until.is_expired() { Some(id) } else { None })
            .collect();

        for id in ids_to_remove {
            self.blacklisted_set.remove(&id);
        }

        for (_, until) in &mut self.blacklisted_set {
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
        let requestor_id = self.requestor_id;

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
                    requestor_id, id, until, usage, *collisions
                );

                self.blacklisted_set.insert(id, until);
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
            "R{}:collisions map = {{ {}  }}",
            requestor_id,
            self.collisions
                .iter()
                .map(|(id, penalty)| format!("{} => {}", id, penalty))
                .fold(String::new(), |acc, s| acc + &s + ", ")
        );

        debug!(
            "R{}:blacklisted map = {{ {}  }}",
            requestor_id,
            self.blacklisted_set
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assign_subtasks() {
        let mut lgrola = LGRola::new(Id::new());
        let s1 = SubTask::new(1.0, 1.0);
        let s2 = SubTask::new(1.0, 1.0);
        let mut task = Task::new();
        task.push_pending(s1);
        task.push_pending(s2);
        let bid1 = (Id::new(), 1.0);
        let bid2 = (Id::new(), 2.0);
        lgrola.ratings.insert(bid1.0, 1.0);
        lgrola.ratings.insert(bid2.0, 1.0);

        assert_eq!(
            lgrola.assign_subtasks(&mut task, vec![bid1, bid2]),
            vec![(bid1.0, s1, 1.0), (bid2.0, s2, 2.0)]
        );
    }

    #[test]
    fn test_complete_task() {
        let mut lgrola = LGRola::new(Id::new());

        for _ in 0..25 {
            let id = Id::new();
            lgrola.ratings.insert(id, 0.75);
            lgrola
                .task_usages
                .entry(id)
                .or_insert(Vec::new())
                .push(50.0);
        }

        let outlier = (Id::new(), 0.1);
        lgrola.ratings.insert(outlier.0, outlier.1);
        lgrola.task_usages.insert(outlier.0, vec![2000.0]);

        lgrola.complete_task();

        assert_eq!(lgrola.blacklisted_set.len(), 1);
        assert!(lgrola.blacklisted_set.contains_key(&outlier.0));
    }
}
