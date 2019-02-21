mod redundancy;

pub use self::redundancy::Redundancy;

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::ops::{Deref, DerefMut};

use log::{debug, warn};
use serde_derive::Deserialize;

// mod ctasks;
// mod lgrola;

// pub use self::ctasks::CTasks;
// pub use self::lgrola::LGRola;

use crate::id::Id;
use crate::task::subtask;
use crate::task::{SubTask, Task};

#[derive(Copy, Clone, Debug, Deserialize)]
pub enum DefenceMechanismType {
    CTasks,
    LGRola,
    Redundancy,
}

impl DefenceMechanismType {
    pub fn into_dm(self, requestor_id: Id) -> Box<dyn DefenceMechanism> {
        match self {
            // DefenceMechanismType::CTasks => Box::new(CTasks::new()),
            // DefenceMechanismType::LGRola => Box::new(LGRola::new()),
            DefenceMechanismType::Redundancy => Box::new(Redundancy::new(requestor_id)),
            _ => unimplemented!(),
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
}
