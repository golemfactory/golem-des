use std::collections::{HashMap, HashSet};

use log::debug;
use statrs::statistics::Statistics;

use super::{BanDuration, DefenceMechanism, DefenceMechanismCommon};

use crate::id::Id;
use crate::task::{SubTask, Task};

#[derive(Debug)]
pub struct CTasks {
    common: DefenceMechanismCommon,
    task_usages: HashMap<Id, Vec<f64>>,
}

impl CTasks {
    const MAX_RATING: f64 = 2.0;

    pub fn new(id: Id) -> CTasks {
        CTasks {
            common: DefenceMechanismCommon::new(id),
            task_usages: HashMap::new(),
        }
    }
}

impl DefenceMechanism for CTasks {
    fn schedule_subtasks(
        &mut self,
        task: &mut Task,
        bids: Vec<(Id, f64)>,
    ) -> Vec<(Id, SubTask, f64)> {
        let self_id = self.as_dm_common().id;
        let mut messages: Vec<(Id, SubTask, f64)> = Vec::new();

        for (provider_id, bid) in bids {
            match task.pop_subtask() {
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
        let ids: HashSet<Id> = self.task_usages.iter().map(|(&id, _)| id).collect();

        let task_usages: HashMap<Id, f64> = self
            .task_usages
            .iter()
            .map(|(&id, usages)| (id, usages.iter().geometric_mean()))
            .collect();

        let mean_usage: f64 = task_usages.iter().map(|(_, usage)| usage).geometric_mean();

        let mean_rating: f64 = self
            .as_dm_common()
            .ratings
            .iter()
            .filter_map(|(id, rating)| if ids.contains(id) { Some(rating) } else { None })
            .geometric_mean();

        let self_id = self.as_dm_common().id;
        debug!(
            "R{}:updating ratings; mean usage: {}, mean rating: {}",
            self_id, mean_usage, mean_rating
        );

        for (id, usage) in task_usages {
            let rating = self
                .as_dm_common_mut()
                .ratings
                .get_mut(&id)
                .expect(&format!("R{}:usage rating for P{} not found!", self_id, id));

            let rating_relative_to_mean = *rating / mean_rating;
            let usage_relative_to_mean = usage / mean_usage;
            let adjustment_factor = (usage_relative_to_mean / rating_relative_to_mean).sqrt();

            let new_rating = *rating * adjustment_factor;

            debug!(
                "R{}:P{} rating updated: {} -> {}",
                self_id, id, *rating, new_rating
            );

            *rating = new_rating;

            if *rating > CTasks::MAX_RATING {
                let until = BanDuration::Indefinitely;

                debug!(
                    "R{}:P{} blacklisted for {:?}, rating = {}",
                    self_id, id, until, rating
                );

                self.as_dm_common_mut().blacklist.insert(id, until);
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    use statrs::assert_almost_eq;

    #[test]
    fn test_task_computed() {
        let id = Id::new();
        let mut ctasks = CTasks::new(id);

        let id1 = Id::new();
        let id2 = Id::new();
        let id3 = Id::new();

        ctasks.as_dm_common_mut().ratings.insert(id1, 0.5);
        ctasks.as_dm_common_mut().ratings.insert(id2, 0.1);
        ctasks.as_dm_common_mut().ratings.insert(id3, 0.75);

        ctasks.task_usages.insert(id1, vec![50.0, 50.0]);
        ctasks.task_usages.insert(id3, vec![75.0]);

        ctasks.task_computed();

        assert_almost_eq!(*ctasks.as_dm_common().ratings.get(&id1).unwrap(), 0.5, 1e-3);
        assert_almost_eq!(
            *ctasks.as_dm_common().ratings.get(&id3).unwrap(),
            0.75,
            1e-3
        );

        ctasks.task_usages.insert(id1, vec![50.0]);
        ctasks.task_usages.insert(id2, vec![2020.0]);
        ctasks.task_usages.insert(id3, vec![75.0]);

        ctasks.task_computed();

        assert_almost_eq!(
            *ctasks.as_dm_common().ratings.get(&id1).unwrap(),
            0.2065,
            1e-3
        );
        assert_almost_eq!(
            *ctasks.as_dm_common().ratings.get(&id2).unwrap(),
            0.5869,
            1e-3
        );
        assert_almost_eq!(
            *ctasks.as_dm_common().ratings.get(&id3).unwrap(),
            0.3097,
            1e-3
        );
    }
}
