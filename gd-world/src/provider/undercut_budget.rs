use std::fmt;

use super::*;
use crate::task::SubTask;

#[derive(Debug)]
pub struct UndercutBudgetProvider {
    epsilon: f64,
    common: ProviderCommon,
}

impl UndercutBudgetProvider {
    pub fn new(min_price: f64, usage_factor: f64, epsilon: f64) -> UndercutBudgetProvider {
        UndercutBudgetProvider::with_id(Id::new(), min_price, usage_factor, epsilon)
    }

    pub fn with_id(
        id: Id,
        min_price: f64,
        usage_factor: f64,
        epsilon: f64,
    ) -> UndercutBudgetProvider {
        UndercutBudgetProvider {
            epsilon: epsilon,
            common: ProviderCommon::new(id, min_price, usage_factor),
        }
    }
}

impl fmt::Display for UndercutBudgetProvider {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            r"UndercutBudgetProvider
            {},
            Epsilon: {},
            ",
            self.common, self.epsilon,
        )
    }
}

impl Provider for UndercutBudgetProvider {
    fn report_usage(&self, subtask: &SubTask, bid: f64) -> f64 {
        subtask.budget / bid * (1.0 - self.epsilon)
    }

    fn into_stats(self: Box<Self>, run_id: u64) -> Stats {
        Stats {
            run_id: run_id,
            behaviour: Behaviour::UndercutBudget,
            min_price: self.common.min_price,
            usage_factor: self.common.usage_factor,
            profit_margin: self.common.profit_margin,
            price: self.common.price(),
            revenue: self.common.revenue,
            num_subtasks_assigned: self.common.num_subtasks_assigned,
            num_subtasks_computed: self.common.num_subtasks_computed,
            num_subtasks_cancelled: self.common.num_subtasks_cancelled,
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
