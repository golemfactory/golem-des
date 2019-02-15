use std::fmt;

use super::*;
use crate::id::Id;
use crate::task::SubTask;

#[derive(Debug)]
pub struct LinearUsageInflationProvider {
    inflation_factor: f64,
    common: ProviderCommon,
}

impl LinearUsageInflationProvider {
    pub fn new(
        min_price: f64,
        usage_factor: f64,
        inflation_factor: f64,
    ) -> LinearUsageInflationProvider {
        LinearUsageInflationProvider::with_id(Id::new(), min_price, usage_factor, inflation_factor)
    }

    pub fn with_id(
        id: Id,
        min_price: f64,
        usage_factor: f64,
        inflation_factor: f64,
    ) -> LinearUsageInflationProvider {
        LinearUsageInflationProvider {
            inflation_factor: inflation_factor,
            common: ProviderCommon::new(id, min_price, usage_factor),
        }
    }
}

impl fmt::Display for LinearUsageInflationProvider {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            r"LinearUsageInflationProvider
            {},
            Inflation factor: {},
            ",
            self.common, self.inflation_factor,
        )
    }
}

impl Provider for LinearUsageInflationProvider {
    fn report_usage(&self, subtask: &SubTask, bid: f64) -> f64 {
        let intercept = self.common.usage_factor() * subtask.nominal_usage;
        let usage = self.common.num_subtasks_computed as f64 * self.inflation_factor + intercept;

        usage.min(subtask.budget / bid)
    }

    fn into_stats(self: Box<Self>, run_id: u64) -> Stats {
        Stats {
            run_id: run_id,
            behaviour: Behaviour::LinearUsageInflation,
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

#[cfg(test)]
mod tests {
    use super::*;

    use statrs::assert_almost_eq;

    use crate::task::SubTask;

    #[test]
    fn linear_usage_inflation_provider_reported_usage() {
        let mut provider = LinearUsageInflationProvider::new(0.1, 0.5, 1.0);
        let subtask = SubTask::new(100.0, 100.0);
        assert_almost_eq!(50.0, provider.report_usage(&subtask, 1.0), 1e-3);

        provider.as_provider_common_mut().num_subtasks_computed = 1;
        assert_almost_eq!(51.0, provider.report_usage(&subtask, 1.0), 1e-3);

        provider.as_provider_common_mut().num_subtasks_computed = 50;
        assert_almost_eq!(100.0, provider.report_usage(&subtask, 1.0), 1e-3);

        provider.as_provider_common_mut().num_subtasks_computed = 51;
        assert_almost_eq!(100.0, provider.report_usage(&subtask, 1.0), 1e-3);

        provider.as_provider_common_mut().num_subtasks_computed = 100;
        assert_almost_eq!(100.0, provider.report_usage(&subtask, 1.0), 1e-3);
    }
}
