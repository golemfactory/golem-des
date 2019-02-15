use std::fmt;

use serde_derive::Deserialize;

static mut BASE_VALUE: usize = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash, Deserialize)]
pub struct Id {
    value: usize,
}

impl Id {
    pub fn new() -> Id {
        unsafe {
            let value = BASE_VALUE;
            BASE_VALUE += 1;

            Id { value: value }
        }
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
