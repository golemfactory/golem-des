#![warn(clippy::all)]

pub mod partition;
pub mod stats;

pub mod prelude {
    pub use crate::partition::*;
    pub use crate::stats::*;
}
