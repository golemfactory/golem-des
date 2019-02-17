use std::collections::HashMap;

use log::debug;

use super::{BanDuration, DefenceMechanism, DefenceMechanismCommon};

use crate::id::Id;
use crate::task::{SubTask, Task};

const REDUNDANCY_FACTOR: usize = 2;

#[derive(Debug)]
struct Verification {
    ptr: usize,
    data: [Option<(Id, f64)>; REDUNDANCY_FACTOR],
}

impl Verification {
    fn new() -> Verification {
        Verification {
            ptr: 0,
            data: [None; REDUNDANCY_FACTOR],
        }
    }

    fn insert(&mut self, id: Id, value: f64) {
        if self.ptr >= REDUNDANCY_FACTOR {
            panic!("inserting out of bounds!");
        }

        self.data[self.ptr] = Some((id, value));
        self.ptr += 1;
    }

    fn is_pending(&self) -> bool {
        self.ptr != REDUNDANCY_FACTOR
    }
}

impl Into<[Option<(Id, f64)>; REDUNDANCY_FACTOR]> for Verification {
    fn into(self) -> [Option<(Id, f64)>; REDUNDANCY_FACTOR] {
        self.data
    }
}

impl std::fmt::Display for Verification {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Verification ({})",
            self.data
                .iter()
                .map(|x| match x {
                    None => String::from("None"),
                    Some((i, x)) => format!("({}, {})", i, x),
                })
                .fold(String::new(), |acc, x| acc + ", " + &x)
        )
    }
}

#[derive(Debug)]
struct VerificationMap {
    verifications: HashMap<Id, Verification>,
}

impl VerificationMap {
    fn new() -> VerificationMap {
        VerificationMap {
            verifications: HashMap::new(),
        }
    }

    fn insert_subtask(&mut self, id: Id) {
        self.verifications.insert(id, Verification::new());
    }

    fn insert_verification(&mut self, subtask_id: Id, provider_id: Id, value: f64) {
        match self.verifications.get_mut(&subtask_id) {
            None => panic!("{} not in {}", subtask_id, self),
            Some(v) => v.insert(provider_id, value),
        }
    }

    fn is_subtask_pending(&self, id: Id) -> bool {
        self.verifications
            .get(&id)
            .expect(&format!(
                "SubTask (id = {}) not found in the verification table",
                id
            ))
            .is_pending()
    }

    fn remove_verifications_for_subtask(
        &mut self,
        id: Id,
    ) -> Option<[Option<(Id, f64)>; REDUNDANCY_FACTOR]> {
        self.verifications.remove(&id).map(|x| x.into())
    }
}

impl std::fmt::Display for VerificationMap {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.verifications
                .iter()
                .map(|(id, v)| id.to_string() + " => " + &v.to_string())
                .fold(String::new(), |acc, x| acc + ", " + &x)
        )
    }
}

#[derive(Debug)]
pub struct Redundancy {
    common: DefenceMechanismCommon,
    verification_map: VerificationMap,
}

impl Redundancy {
    pub fn new(id: Id) -> Redundancy {
        Redundancy {
            common: DefenceMechanismCommon::new(id),
            verification_map: VerificationMap::new(),
        }
    }
}

impl DefenceMechanism for Redundancy {
    fn schedule_subtasks(
        &mut self,
        task: &mut Task,
        bids: Vec<(Id, f64)>,
    ) -> Vec<(Id, SubTask, f64)> {
        let self_id = self.as_dm_common().id;
        let mut messages: Vec<(Id, SubTask, f64)> = Vec::new();

        for chunk in bids.chunks_exact(REDUNDANCY_FACTOR) {
            match task.pop_subtask() {
                Some(subtask) => {
                    for &(provider_id, bid) in chunk {
                        debug!(
                            "R{}:sending {} to P{} for {}",
                            self_id, subtask, provider_id, bid
                        );

                        messages.push((provider_id, subtask, bid));
                    }

                    self.verification_map.insert_subtask(subtask.id());
                }
                None => break,
            }
        }

        messages
    }

    fn subtask_computed(
        &mut self,
        subtask: &SubTask,
        provider_id: &Id,
        reported_usage: f64,
        _bid: f64,
    ) {
        let self_id = self.as_dm_common().id;
        let usage_factor = self
            .as_dm_common()
            .ratings
            .get(&provider_id)
            .expect(&format!(
                "R{}:usage rating for P{} not found!",
                self_id, provider_id
            ));

        debug!(
            "R{}:inserting verification for {}: ({}, {})",
            self_id,
            subtask,
            provider_id,
            reported_usage / usage_factor
        );

        self.verification_map.insert_verification(
            subtask.id(),
            *provider_id,
            reported_usage / usage_factor,
        );

        debug!("R{}:verification map {}", self_id, self.verification_map);

        if self.verification_map.is_subtask_pending(subtask.id()) {
            return;
        }

        let verification = self
            .verification_map
            .remove_verifications_for_subtask(subtask.id())
            .expect(&format!(
                "R{}:{} verified but misses verification data",
                self_id, subtask
            ));
        let d1 = verification[0].unwrap().1;
        let d2 = verification[1].unwrap().1;

        let mut update_rating = |(id1, d1): (Id, f64), (_, d2): (Id, f64)| {
            let usage_factor = self
                .as_dm_common_mut()
                .ratings
                .get_mut(&id1)
                .expect(&format!(
                    "R{}:usage rating for P{} not found!",
                    self_id, provider_id
                ));
            let old_rating = *usage_factor;
            *usage_factor *= d1 / d2;

            debug!(
                "R{}:P{} failed verification, rating {} => {}",
                self_id, id1, old_rating, *usage_factor
            );

            if *usage_factor >= 2.0 {
                let duration = BanDuration::Indefinitely;

                debug!("R{}:P{} blacklisted for {:?}", self_id, id1, duration);

                self.as_dm_common_mut().blacklist.insert(id1, duration);
            }
        };

        if (d1 - d2).abs() >= 1e-3 {
            if d1 > d2 {
                update_rating(verification[0].unwrap(), verification[1].unwrap())
            } else if d1 < d2 {
                update_rating(verification[1].unwrap(), verification[0].unwrap())
            }
        }
    }

    fn task_computed(&mut self) {
        println!("{}", self.verification_map);
    }

    fn as_dm_common(&self) -> &DefenceMechanismCommon {
        &self.common
    }

    fn as_dm_common_mut(&mut self) -> &mut DefenceMechanismCommon {
        &mut self.common
    }
}
