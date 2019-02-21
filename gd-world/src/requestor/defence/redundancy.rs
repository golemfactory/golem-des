use super::{DefenceMechanism, DefenceMechanismCommon};

use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use log::debug;

use crate::id::Id;
use crate::task::subtask;
use crate::task::{SubTask, Task};

type VerificationResult = Option<(Id, f64)>;

const REDUNDANCY_FACTOR: usize = 2;

#[derive(Debug)]
struct VerificationMap {
    map: HashMap<Id, Vec<VerificationResult>>,
}

impl VerificationMap {
    fn new() -> VerificationMap {
        VerificationMap {
            map: HashMap::new(),
        }
    }

    fn insert_key(&mut self, key: Id) {
        self.map.insert(key, Vec::with_capacity(REDUNDANCY_FACTOR));
    }

    fn insert_verification(&mut self, key: &Id, res: VerificationResult) -> Option<Vec<(Id, f64)>> {
        if !self.map.contains_key(key) {
            panic!("verification key not found");
        }

        self.map.get_mut(key).unwrap().push(res);

        if self.map.get(key).unwrap().len() == REDUNDANCY_FACTOR {
            Some(
                self.map
                    .remove(key)
                    .unwrap()
                    .into_iter()
                    .filter_map(|v| v)
                    .collect(),
            )
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Redundancy {
    common: DefenceMechanismCommon,
    verification_map: VerificationMap,
}

impl Redundancy {
    pub fn new(requestor_id: Id) -> Redundancy {
        Redundancy {
            common: DefenceMechanismCommon::new(requestor_id),
            verification_map: VerificationMap::new(),
        }
    }

    fn update_rating(&mut self, p1: (Id, f64), p2: (Id, f64)) {
        let requestor_id = self.requestor_id;
        let (id1, d1) = p1;
        let (_, d2) = p2;
        let old_rating = self.get_provider_rating(&id1);
        let new_rating = old_rating * d1 / d2;

        debug!(
            "R{}:P{} failed verification, rating {} => {}",
            requestor_id, id1, old_rating, new_rating
        );

        self.update_provider_rating(&id1, new_rating);
    }
}

impl DefenceMechanism for Redundancy {
    fn assign_subtasks(
        &mut self,
        task: &mut Task,
        bids: Vec<(Id, f64)>,
    ) -> Vec<(Id, SubTask, f64)> {
        let bids = self.filter_offers(bids);
        let bids = self.rank_offers(bids);

        let mut messages: Vec<(Id, SubTask, f64)> = Vec::new();

        for chunk in bids.chunks_exact(REDUNDANCY_FACTOR) {
            match task.pop_pending() {
                Some(subtask) => {
                    for &(provider_id, bid) in chunk {
                        debug!("sending {} to P{} for {}", subtask, provider_id, bid);

                        messages.push((provider_id, subtask, bid));
                    }

                    self.verification_map.insert_key(*subtask.id());
                }
                None => break,
            }
        }

        messages
    }

    fn verify_subtask(
        &mut self,
        subtask: &SubTask,
        provider_id: &Id,
        reported_usage: Option<f64>,
    ) -> subtask::Status {
        let ver_res = reported_usage.map(|usage| {
            let rating = self.ratings.get(&provider_id).expect("rating not found");

            (*provider_id, usage / rating)
        });

        if let Some(vers) = self
            .verification_map
            .insert_verification(subtask.id(), ver_res)
        {
            if vers.len() > 0 {
                if vers.len() == REDUNDANCY_FACTOR {
                    if vers[0].1 > vers[1].1 {
                        self.update_rating(vers[0], vers[1]);
                    } else {
                        self.update_rating(vers[1], vers[0]);
                    }
                }

                subtask::Status::Done
            } else {
                subtask::Status::Cancelled
            }
        } else {
            subtask::Status::Pending
        }
    }

    fn complete_task(&mut self) {}

    fn as_dm_common(&self) -> &DefenceMechanismCommon {
        &self.common
    }

    fn as_dm_common_mut(&mut self) -> &mut DefenceMechanismCommon {
        &mut self.common
    }
}

impl Deref for Redundancy {
    type Target = DefenceMechanismCommon;

    fn deref(&self) -> &DefenceMechanismCommon {
        self.as_dm_common()
    }
}

impl DerefMut for Redundancy {
    fn deref_mut(&mut self) -> &mut DefenceMechanismCommon {
        self.as_dm_common_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use statrs::assert_almost_eq;

    #[test]
    fn test_insert_verification() {
        let mut vmap = VerificationMap::new();
        let id = Id::new();
        vmap.insert_key(id);

        let id1 = Id::new();
        let id2 = Id::new();

        assert_eq!(vmap.insert_verification(&id, Some((id1, 1.0))), None);
        assert_eq!(
            vmap.insert_verification(&id, Some((id2, 1.0))),
            Some(vec![(id1, 1.0), (id2, 1.0)])
        );
        assert_eq!(vmap.map.get(&id), None);

        vmap.insert_key(id);

        assert_eq!(vmap.insert_verification(&id, Some((id1, 1.0))), None);
        assert_eq!(vmap.insert_verification(&id, None), Some(vec![(id1, 1.0)]));
        assert_eq!(vmap.map.get(&id), None);

        vmap.insert_key(id);

        assert_eq!(vmap.insert_verification(&id, None), None);
        assert_eq!(
            vmap.insert_verification(&id, Some((id2, 1.0))),
            Some(vec![(id2, 1.0)])
        );
        assert_eq!(vmap.map.get(&id), None);

        vmap.insert_key(id);

        assert_eq!(vmap.insert_verification(&id, None), None);
        assert_eq!(vmap.insert_verification(&id, None), Some(vec![]));
        assert_eq!(vmap.map.get(&id), None);
    }

    #[test]
    fn test_update_rating() {
        let mut redundancy = Redundancy::new(Id::new());
        let p1 = (Id::new(), 0.25, 25.0); // (provider_id, rating, usage)
        let p2 = (Id::new(), 0.75, 75.0);
        redundancy.ratings.insert(p1.0, p1.1);
        redundancy.ratings.insert(p2.0, p2.1);

        redundancy.update_rating((p1.0, p1.2 / p1.1), (p2.0, p2.2 / p2.1));

        assert_almost_eq!(*redundancy.ratings.get(&p1.0).unwrap(), 0.25, 1e-5);
        assert_almost_eq!(*redundancy.ratings.get(&p2.0).unwrap(), 0.75, 1e-5);
        assert!(redundancy.blacklisted_set.is_empty());

        redundancy.update_rating((p1.0, 200.0), (p2.0, p2.2 / p2.1));

        assert_almost_eq!(*redundancy.ratings.get(&p1.0).unwrap(), 0.5, 1e-5);
        assert_almost_eq!(*redundancy.ratings.get(&p2.0).unwrap(), 0.75, 1e-5);
        assert!(redundancy.blacklisted_set.is_empty());

        redundancy.update_rating((p1.0, 400.0), (p2.0, p2.2 / p2.1));

        assert_almost_eq!(*redundancy.ratings.get(&p1.0).unwrap(), 2.0, 1e-5);
        assert_almost_eq!(*redundancy.ratings.get(&p2.0).unwrap(), 0.75, 1e-5);
        assert!(redundancy.blacklisted_set.contains(&p1.0));
    }

    #[test]
    fn test_assign_subtasks() {
        let mut redundancy = Redundancy::new(Id::new());
        let subtask = SubTask::new(1.0, 1.0);
        let mut task = Task::new();
        task.push_pending(subtask);
        let bid1 = (Id::new(), 1.0);
        let bid2 = (Id::new(), 2.0);
        redundancy.ratings.insert(bid1.0, 1.0);
        redundancy.ratings.insert(bid2.0, 1.0);

        assert_eq!(
            redundancy.assign_subtasks(&mut task, vec![bid1, bid2]),
            vec![(bid1.0, subtask, 1.0), (bid2.0, subtask, 2.0)]
        );
    }

    #[test]
    fn test_verify_subtask_successful() {
        let mut redundancy = Redundancy::new(Id::new());

        let p1 = (Id::new(), 0.25, 100.0);
        let p2 = (Id::new(), 0.75, 75.0);
        redundancy.ratings.insert(p1.0, p1.1);
        redundancy.ratings.insert(p2.0, p2.1);

        let subtask = SubTask::new(100.0, 100.0);
        redundancy.verification_map.insert_key(*subtask.id());

        assert_eq!(
            redundancy.verify_subtask(&subtask, &p1.0, Some(p1.2)),
            subtask::Status::Pending
        );
        assert_eq!(
            redundancy.verify_subtask(&subtask, &p2.0, Some(p2.2)),
            subtask::Status::Done
        );
    }

    #[test]
    fn test_verify_subtask_partial_success() {
        {
            let mut redundancy = Redundancy::new(Id::new());

            let p1 = (Id::new(), 0.25, 100.0);
            let p2 = (Id::new(), 0.75, 75.0);
            redundancy.ratings.insert(p1.0, p1.1);
            redundancy.ratings.insert(p2.0, p2.1);

            let subtask = SubTask::new(100.0, 100.0);
            redundancy.verification_map.insert_key(*subtask.id());

            assert_eq!(
                redundancy.verify_subtask(&subtask, &p1.0, Some(p1.2)),
                subtask::Status::Pending
            );
            assert_eq!(
                redundancy.verify_subtask(&subtask, &p1.0, None),
                subtask::Status::Done
            );
        }

        {
            let mut redundancy = Redundancy::new(Id::new());

            let p1 = (Id::new(), 0.25, 100.0);
            let p2 = (Id::new(), 0.75, 75.0);
            redundancy.ratings.insert(p1.0, p1.1);
            redundancy.ratings.insert(p2.0, p2.1);

            let subtask = SubTask::new(100.0, 100.0);
            redundancy.verification_map.insert_key(*subtask.id());

            assert_eq!(
                redundancy.verify_subtask(&subtask, &p1.0, None),
                subtask::Status::Pending
            );
            assert_eq!(
                redundancy.verify_subtask(&subtask, &p1.0, Some(p2.2)),
                subtask::Status::Done
            );
        }
    }

    #[test]
    fn test_verify_subtask_failed() {
        let mut redundancy = Redundancy::new(Id::new());

        let p1 = (Id::new(), 0.25, 100.0);
        let p2 = (Id::new(), 0.75, 75.0);
        redundancy.ratings.insert(p1.0, p1.1);
        redundancy.ratings.insert(p2.0, p2.1);

        let subtask = SubTask::new(100.0, 100.0);
        redundancy.verification_map.insert_key(*subtask.id());

        assert_eq!(
            redundancy.verify_subtask(&subtask, &p1.0, None),
            subtask::Status::Pending
        );
        assert_eq!(
            redundancy.verify_subtask(&subtask, &p2.0, None),
            subtask::Status::Cancelled
        );
    }
}
