use std::fmt;
use std::sync::atomic::{ATOMIC_USIZE_INIT, AtomicUsize, Ordering};

use serde_derive::Deserialize;

static BASE_VALUE: AtomicUsize = ATOMIC_USIZE_INIT;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Deserialize)]
pub struct Id {
    value: usize,
}

impl Id {
    pub fn new() -> Id {
        let value = BASE_VALUE.fetch_add(1, Ordering::SeqCst);
        Id { value: value }
    }

    pub fn value(&self) -> usize {
        self.value
    }
}

impl Default for Id {
    fn default() -> Id {
        Id::new()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
