use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_derive::Deserialize;

static BASE_VALUE: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Deserialize)]
pub struct Id {
    value: usize,
}

impl Id {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn value(self) -> usize {
        self.value
    }
}

impl Default for Id {
    fn default() -> Self {
        let value = BASE_VALUE.fetch_add(1, Ordering::SeqCst);
        Self { value }
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}
