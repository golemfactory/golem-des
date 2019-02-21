use std::collections::{HashMap, HashSet};
use std::ops::{Deref, DerefMut};

use log::debug;
use statrs::statistics::Statistics;

use super::{DefenceMechanism, DefenceMechanismCommon};

use crate::id::Id;
use crate::task::subtask;
use crate::task::{SubTask, Task};

#[derive(Debug)]
pub struct CTasks {
    common: DefenceMechanismCommon,
    task_usages: HashMap<Id, Vec<f64>>,
}

impl CTasks {
    pub fn new(id: Id) -> CTasks {
        CTasks {
            common: DefenceMechanismCommon::new(id),
            task_usages: HashMap::new(),
        }
    }
}

impl DefenceMechanism for CTasks {
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
        let ids: HashSet<Id> = self.task_usages.iter().map(|(&id, _)| id).collect();

        let task_usages: HashMap<Id, f64> = self
            .task_usages
            .iter()
            .map(|(&id, usages)| (id, usages.iter().geometric_mean()))
            .collect();

        let mean_usage: f64 = task_usages.iter().map(|(_, usage)| usage).geometric_mean();

        let mean_rating: f64 = self
            .ratings
            .iter()
            .filter_map(|(id, rating)| if ids.contains(id) { Some(rating) } else { None })
            .geometric_mean();

        debug!(
            "R{}:updating ratings; mean usage: {}, mean rating: {}",
            self.requestor_id, mean_usage, mean_rating
        );

        for (id, usage) in task_usages {
            let rating = self.get_provider_rating(&id);
            let rating_relative_to_mean = rating / mean_rating;
            let usage_relative_to_mean = usage / mean_usage;
            let adjustment_factor = (usage_relative_to_mean / rating_relative_to_mean).sqrt();

            let new_rating = rating * adjustment_factor;

            debug!(
                "R{}:P{} rating updated: {} -> {}",
                self.requestor_id, id, rating, new_rating
            );

            self.update_provider_rating(&id, new_rating);
        }

        self.task_usages.clear();
    }

    fn as_dm_common(&self) -> &DefenceMechanismCommon {
        &self.common
    }

    fn as_dm_common_mut(&mut self) -> &mut DefenceMechanismCommon {
        &mut self.common
    }
}

impl Deref for CTasks {
    type Target = DefenceMechanismCommon;

    fn deref(&self) -> &DefenceMechanismCommon {
        self.as_dm_common()
    }
}

impl DerefMut for CTasks {
    fn deref_mut(&mut self) -> &mut DefenceMechanismCommon {
        self.as_dm_common_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use statrs::assert_almost_eq;

    #[test]
    fn test_assign_subtasks() {
        let mut ctasks = CTasks::new(Id::new());
        let s1 = SubTask::new(1.0, 1.0);
        let s2 = SubTask::new(1.0, 1.0);
        let mut task = Task::new();
        task.push_pending(s1);
        task.push_pending(s2);
        let bid1 = (Id::new(), 1.0);
        let bid2 = (Id::new(), 2.0);
        ctasks.ratings.insert(bid1.0, 1.0);
        ctasks.ratings.insert(bid2.0, 1.0);

        assert_eq!(
            ctasks.assign_subtasks(&mut task, vec![bid1, bid2]),
            vec![(bid1.0, s1, 1.0), (bid2.0, s2, 2.0)]
        );
    }

    #[test]
    fn test_complete_task() {
        let mut ctasks = CTasks::new(Id::new());

        let id1 = Id::new();
        let id2 = Id::new();
        let id3 = Id::new();

        ctasks.ratings.insert(id1, 0.5);
        ctasks.ratings.insert(id2, 0.1);
        ctasks.ratings.insert(id3, 0.75);

        ctasks.task_usages.insert(id1, vec![50.0, 50.0]);
        ctasks.task_usages.insert(id3, vec![75.0]);

        ctasks.complete_task();

        assert_almost_eq!(*ctasks.ratings.get(&id1).unwrap(), 0.5, 1e-3);
        assert_almost_eq!(*ctasks.ratings.get(&id3).unwrap(), 0.75, 1e-3);

        ctasks.task_usages.insert(id1, vec![50.0]);
        ctasks.task_usages.insert(id2, vec![2020.0]);
        ctasks.task_usages.insert(id3, vec![75.0]);

        ctasks.complete_task();

        assert_almost_eq!(*ctasks.ratings.get(&id1).unwrap(), 0.2065, 1e-3);
        assert_almost_eq!(*ctasks.ratings.get(&id2).unwrap(), 0.5869, 1e-3);
        assert_almost_eq!(*ctasks.ratings.get(&id3).unwrap(), 0.3097, 1e-3);
    }

}
