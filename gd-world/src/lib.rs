#![warn(clippy::all)]

pub mod id;
pub mod logger;
pub mod provider;
pub mod requestor;
pub mod task;
pub mod world;

pub mod prelude {
    pub use crate::id::Id;
    pub use crate::provider::{
        LinearUsageInflationProvider, Provider, RegularProvider, UndercutBudgetProvider,
    };
    pub use crate::requestor::{Requestor, TaskQueue};
    pub use crate::task::{SubTask, Task};
    pub use crate::world::World;
}
