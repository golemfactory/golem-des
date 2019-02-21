use std::fmt;

use gd_world_derive::DerefProviderCommon;

use super::*;
use crate::task::SubTask;

#[derive(Debug, DerefProviderCommon)]
pub struct RegularProvider {
    common: ProviderCommon,
}

impl RegularProvider {
    pub fn new(min_price: f64, usage_factor: f64) -> RegularProvider {
        RegularProvider::with_id(Id::new(), min_price, usage_factor)
    }

    pub fn with_id(id: Id, min_price: f64, usage_factor: f64) -> RegularProvider {
        RegularProvider {
            common: ProviderCommon::new(id, min_price, usage_factor),
        }
    }
}

impl fmt::Display for RegularProvider {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            r"RegularProvider
            {}
            ",
            self.common,
        )
    }
}

impl Provider for RegularProvider {
    fn report_usage(&self, subtask: &SubTask, _bid: f64) -> f64 {
        subtask.nominal_usage * self.usage_factor()
    }

    fn into_stats(self: Box<Self>, run_id: u64) -> Stats {
        Stats {
            run_id: run_id,
            behaviour: Behaviour::Regular,
            min_price: self.min_price,
            usage_factor: self.usage_factor,
            profit_margin: self.profit_margin,
            price: self.price(),
            revenue: self.revenue,
            num_subtasks_assigned: self.num_subtasks_assigned,
            num_subtasks_computed: self.num_subtasks_computed,
            num_subtasks_cancelled: self.num_subtasks_cancelled,
        }
    }

    fn as_provider_common(&self) -> &ProviderCommon {
        &self.common
    }

    fn as_provider_common_mut(&mut self) -> &mut ProviderCommon {
        &mut self.common
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
