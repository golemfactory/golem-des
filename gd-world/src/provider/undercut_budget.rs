use std::fmt;

use gd_world_derive::DerefProviderCommon;

use super::*;
use crate::task::SubTask;

#[derive(Debug, DerefProviderCommon)]
pub struct UndercutBudgetProvider {
    epsilon: f64,
    common: ProviderCommon,
}

impl UndercutBudgetProvider {
    pub fn new(min_price: f64, usage_factor: f64, epsilon: f64) -> Self {
        Self::with_id(Id::new(), min_price, usage_factor, epsilon)
    }

    pub fn with_id(id: Id, min_price: f64, usage_factor: f64, epsilon: f64) -> Self {
        Self {
            epsilon,
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
            run_id,
            behaviour: Behaviour::UndercutBudget,
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

#[cfg(test)]
mod tests {
    use super::*;

    use statrs::assert_almost_eq;

    use crate::task::SubTask;

    #[test]
    fn report_usage() {
        let mut provider = UndercutBudgetProvider::new(0.1, 0.1, 0.0);
        let subtask = SubTask::new(100.0, 100.0);

        assert_almost_eq!(provider.report_usage(&subtask, 1.0), 100.0, 1e-6);
        assert_almost_eq!(provider.report_usage(&subtask, 0.1) * 0.1, 100.0, 1e-6);

        provider.epsilon = 0.5;

        assert_almost_eq!(provider.report_usage(&subtask, 1.0), 50.0, 1e-6);
        assert_almost_eq!(provider.report_usage(&subtask, 0.1) * 0.1, 50.0, 1e-6);
    }
}
