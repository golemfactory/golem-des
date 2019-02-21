mod ctasks;
mod redundancy;

pub use self::ctasks::CTasks;
pub use self::redundancy::Redundancy;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::ops::{Deref, DerefMut};

use log::{debug, warn};
use serde_derive::Deserialize;

use crate::id::Id;
use crate::task::subtask;
use crate::task::{SubTask, Task};

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum DefenceMechanismType {
    CTasks,
    Redundancy,
}

impl DefenceMechanismType {
    pub fn into_dm(self, requestor_id: Id) -> Box<dyn DefenceMechanism> {
        match self {
            DefenceMechanismType::CTasks => Box::new(CTasks::new(requestor_id)),
            DefenceMechanismType::Redundancy => Box::new(Redundancy::new(requestor_id)),
        }
    }
}

pub trait DefenceMechanism: fmt::Debug {
    fn assign_subtasks(&mut self, task: &mut Task, bids: Vec<(Id, f64)>)
        -> Vec<(Id, SubTask, f64)>;

    fn verify_subtask(
        &mut self,
        subtask: &SubTask,
        provider_id: &Id,
        reported_usage: Option<f64>,
    ) -> subtask::Status;

    fn complete_task(&mut self);

    fn as_dm_common(&self) -> &DefenceMechanismCommon;
    fn as_dm_common_mut(&mut self) -> &mut DefenceMechanismCommon;
}

impl Deref for DefenceMechanism {
    type Target = DefenceMechanismCommon;

    fn deref(&self) -> &DefenceMechanismCommon {
        self.as_dm_common()
    }
}

impl DerefMut for DefenceMechanism {
    fn deref_mut(&mut self) -> &mut DefenceMechanismCommon {
        self.as_dm_common_mut()
    }
}

#[derive(Debug)]
pub struct DefenceMechanismCommon {
    requestor_id: Id,
    ratings: HashMap<Id, f64>,
    blacklisted_set: HashSet<Id>,
}

impl DefenceMechanismCommon {
    const MAX_RATING: f64 = 2.0;

    fn new(requestor_id: Id) -> DefenceMechanismCommon {
        DefenceMechanismCommon {
            requestor_id: requestor_id,
            ratings: HashMap::new(),
            blacklisted_set: HashSet::new(),
        }
    }

    pub fn insert_provider_rating(&mut self, provider_id: &Id, reported_usage: f64) {
        if let Some(old_rating) = self.ratings.insert(*provider_id, reported_usage) {
            warn!(
                "R{}:rating for P{} already existed, replacing: {} => {}",
                self.requestor_id, provider_id, old_rating, reported_usage
            )
        }
    }

    fn get_provider_rating(&self, provider_id: &Id) -> f64 {
        *self.ratings.get(provider_id).expect("rating not found")
    }

    fn update_provider_rating(&mut self, provider_id: &Id, new_rating: f64) {
        let rating = self.ratings.get_mut(provider_id).expect("rating not found");
        *rating = new_rating;

        if *rating >= Self::MAX_RATING {
            debug!("R{}:P{} blacklisted", self.requestor_id, provider_id);

            self.blacklisted_set.insert(*provider_id);
        }
    }

    fn filter_offers(&self, bids: Vec<(Id, f64)>) -> Vec<(Id, f64)> {
        debug!(
            "R{}:offers before filtering = {{ {} }}",
            self.requestor_id,
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
            self.requestor_id,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rank_offers() {
        let mut dm = DefenceMechanismCommon::new(Id::new());
        let bid1 = (Id::new(), 2.5); // (provider_id, bid/offer)
        let bid2 = (Id::new(), 0.5);
        dm.ratings.insert(bid1.0, 0.25);
        dm.ratings.insert(bid2.0, 0.75);

        assert_eq!(dm.rank_offers(vec![bid1, bid2]), vec![bid2, bid1]);
    }

    #[test]
    fn test_filter_offers() {
        let mut dm = DefenceMechanismCommon::new(Id::new());
        let bid1 = (Id::new(), 1.0); // (provider_id, bid/offer)
        let bid2 = (Id::new(), 2.0);

        assert!(dm.blacklisted_set.is_empty());
        assert_eq!(dm.filter_offers(vec![bid1, bid2]), vec![bid1, bid2]);

        dm.blacklisted_set.insert(bid1.0);

        assert!(!dm.blacklisted_set.is_empty());
        assert_eq!(dm.filter_offers(vec![bid1, bid2]), vec![bid2]);
    }

    #[test]
    fn test_update_provider_rating() {
        let mut dm = DefenceMechanismCommon::new(Id::new());
        let provider = (Id::new(), 0.25);

        dm.ratings.insert(provider.0, provider.1);
        dm.update_provider_rating(&provider.0, 2.0);

        assert!(dm.blacklisted_set.contains(&provider.0));
    }
}
