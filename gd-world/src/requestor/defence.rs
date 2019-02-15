use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::ops::{Deref, DerefMut};

use num_traits::{NumAssign, NumCast};
use serde_derive::Deserialize;

mod ctasks;
mod lgrola;
mod redundancy;

pub use self::ctasks::CTasks;
pub use self::lgrola::LGRola;
pub use self::redundancy::Redundancy;

use crate::id::Id;
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
            DefenceMechanismType::CTasks => Box::new(CTasks::new(requestor_id)),
            DefenceMechanismType::LGRola => Box::new(LGRola::new(requestor_id)),
            DefenceMechanismType::Redundancy => Box::new(Redundancy::new(requestor_id)),
        }
    }
}

pub trait DefenceMechanism: fmt::Debug {
    fn schedule_subtasks(
        &mut self,
        subtask_queue: &mut VecDeque<Task>,
        bids: Vec<(Id, f64)>,
    ) -> Vec<(Id, SubTask, f64)>;

    fn subtask_computed(
        &mut self,
        subtask: &SubTask,
        provider_id: &Id,
        reported_usage: f64,
        bid: f64,
    );

    fn task_computed(&mut self);

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
enum BanDuration<T>
where
    T: fmt::Debug + NumAssign + NumCast,
{
    Until(T),
    Indefinitely,
}

impl<T> BanDuration<T>
where
    T: fmt::Debug + NumAssign + NumCast,
{
    fn is_expired(&self) -> bool {
        match self {
            BanDuration::Until(value) => {
                *value == NumCast::from(0).expect("could not convert int to T")
            }
            BanDuration::Indefinitely => false,
        }
    }

    fn decrement(&mut self) {
        if let BanDuration::Until(ref mut value) = *self {
            *value -= NumCast::from(1).expect("could not convert int to T")
        }
    }
}

#[derive(Debug)]
pub struct DefenceMechanismCommon {
    id: Id,
    ratings: HashMap<Id, f64>,
    blacklist: HashMap<Id, BanDuration<i64>>,
}

impl DefenceMechanismCommon {
    fn new(id: Id) -> DefenceMechanismCommon {
        DefenceMechanismCommon {
            id: id,
            ratings: HashMap::new(),
            blacklist: HashMap::new(),
        }
    }

    pub fn insert_provider_rating(&mut self, provider_id: &Id, reported_usage: f64) {
        if let Some(_) = self.ratings.insert(*provider_id, reported_usage) {
            panic!("R{}:rating for P{} already existed!", self.id, provider_id);
        }
    }

    pub fn is_blacklisted(&self, provider_id: &Id) -> bool {
        self.blacklist.contains_key(provider_id)
    }

    pub fn get_rating(&self, provider_id: &Id) -> Option<&f64> {
        self.ratings.get(provider_id)
    }

    pub fn get_rating_mut(&mut self, provider_id: &Id) -> Option<&mut f64> {
        self.ratings.get_mut(provider_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_provider_rating() {
        let id = Id::new();
        let mut bc = DefenceMechanismCommon::new(id);
        assert!(bc.ratings.is_empty());

        let provider_id = Id::new();
        bc.insert_provider_rating(&provider_id, 0.5);
        assert_eq!(*bc.ratings.get(&provider_id).unwrap(), 0.5);

        let provider_id = Id::new();
        bc.insert_provider_rating(&provider_id, 2.5);
        assert_eq!(*bc.ratings.get(&provider_id).unwrap(), 2.5);
    }
}
