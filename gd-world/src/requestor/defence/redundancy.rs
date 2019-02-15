use std::collections::{HashMap, VecDeque};

use log::debug;

use super::{BanDuration, DefenceMechanism, DefenceMechanismCommon};

use crate::id::Id;
use crate::task::{SubTask, Task};

#[derive(Debug)]
pub struct Redundancy {
    common: DefenceMechanismCommon,
    verifications: HashMap<SubTask, Vec<(Id, f64)>>,
}

impl Redundancy {
    pub fn new(id: Id) -> Redundancy {
        Redundancy {
            common: DefenceMechanismCommon::new(id),
            verifications: HashMap::new(),
        }
    }
}

impl DefenceMechanism for Redundancy {
    fn schedule_subtasks(
        &mut self,
        subtask_queue: &mut VecDeque<Task>,
        bids: Vec<(Id, f64)>,
    ) -> Vec<(Id, SubTask, f64)> {
        let self_id = self.as_dm_common().id;
        let mut messages: Vec<(Id, SubTask, f64)> = Vec::new();

        for chunk in bids.chunks_exact(2) {
            match subtask_queue
                .front_mut()
                .expect(&format!("R{}:task not found!", self_id))
                .pop_subtask()
            {
                Some(subtask) => {
                    for &(provider_id, bid) in chunk {
                        debug!(
                            "R{}:sending {} to P{} for {}",
                            self_id, subtask, provider_id, bid
                        );

                        messages.push((provider_id, subtask, bid));
                        self.verifications.entry(subtask).or_insert(Vec::new());
                    }
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
        if let Some((subtask, mut verification)) = self.verifications.remove_entry(&subtask) {
            let self_id = self.as_dm_common().id;
            let usage_factor = self
                .as_dm_common()
                .ratings
                .get(&provider_id)
                .expect(&format!(
                    "R{}:usage rating for P{} not found!",
                    self_id, provider_id
                ));
            verification.push((*provider_id, reported_usage / usage_factor));

            let dist = move |x1: f64, x2: f64| (x1 - x2).abs() > 1e-3;

            if verification.len() == 2 {
                let d1 = verification[0].1;
                let d2 = verification[1].1;

                if dist(d1, d2) && d1 > d2 {
                    let id = verification[0].0;
                    let usage_factor =
                        self.as_dm_common_mut()
                            .ratings
                            .get_mut(&id)
                            .expect(&format!(
                                "R{}:usage rating for P{} not found!",
                                self_id, provider_id
                            ));
                    let old_rating = *usage_factor;
                    *usage_factor *= d1 / d2;

                    debug!(
                        "R{}:P{} failed verification, rating {} => {}",
                        self_id, id, old_rating, *usage_factor
                    );

                    if *usage_factor >= 2.0 {
                        let duration = BanDuration::Indefinitely;

                        debug!("R{}:P{} blacklisted for {:?}", self_id, id, duration);

                        self.as_dm_common_mut().blacklist.insert(id, duration);
                    }
                } else if dist(d1, d2) && d1 < d2 {
                    let id = verification[1].0;
                    let usage_factor =
                        self.as_dm_common_mut()
                            .ratings
                            .get_mut(&id)
                            .expect(&format!(
                                "R{}:usage rating for P{} not found!",
                                self_id, provider_id
                            ));
                    let old_rating = *usage_factor;
                    *usage_factor *= d2 / d1;

                    debug!(
                        "R{}:P{} failed verification, rating {} => {}",
                        self_id, id, old_rating, *usage_factor
                    );

                    if *usage_factor >= 2.0 {
                        let duration = BanDuration::Indefinitely;

                        debug!("R{}:P{} blacklisted for {:?}", self_id, id, duration);

                        self.as_dm_common_mut().blacklist.insert(id, duration);
                    }
                }
            } else {
                self.verifications.insert(subtask, verification);
            }
        }
    }

    fn task_computed(&mut self) {}

    fn as_dm_common(&self) -> &DefenceMechanismCommon {
        &self.common
    }

    fn as_dm_common_mut(&mut self) -> &mut DefenceMechanismCommon {
        &mut self.common
    }
}
