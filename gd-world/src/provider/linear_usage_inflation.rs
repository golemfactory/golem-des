use std::fmt;
use std::marker::PhantomData;

use gd_world_derive::DerefProviderCommon;

use super::*;
use crate::id::Id;
use crate::task::SubTask;

#[derive(Debug, DerefProviderCommon)]
pub struct LinearUsageInflationProvider<Rng>
where
    Rng: rand::Rng + 'static,
{
    inflation_factor: f64,
    common: ProviderCommon,
    phantom: PhantomData<Rng>,
}

impl<Rng> LinearUsageInflationProvider<Rng>
where
    Rng: rand::Rng + 'static,
{
    pub fn new(min_price: f64, usage_factor: f64, inflation_factor: f64) -> Self {
        Self::with_id(Id::new(), min_price, usage_factor, inflation_factor)
    }

    pub fn with_id(id: Id, min_price: f64, usage_factor: f64, inflation_factor: f64) -> Self {
        Self {
            inflation_factor,
            common: ProviderCommon::new(id, min_price, usage_factor),
            phantom: PhantomData,
        }
    }
}

impl<Rng> fmt::Display for LinearUsageInflationProvider<Rng>
where
    Rng: rand::Rng + 'static,
{
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

impl<Rng> Provider for LinearUsageInflationProvider<Rng>
where
    Rng: rand::Rng + 'static,
{
    type Rng = Rng;

    fn report_usage(&self, _rng: &mut Self::Rng, subtask: &SubTask, bid: f64) -> f64 {
        let intercept = self.usage_factor() * subtask.nominal_usage;
        let usage = self.num_subtasks_computed as f64 * self.inflation_factor + intercept;

        usage.min(subtask.budget / bid)
    }

    fn into_stats(self: Box<Self>, run_id: u64) -> Stats {
        Stats {
            run_id,
            behaviour: Behaviour::LinearUsageInflation,
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

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use statrs::assert_almost_eq;

    use crate::task::SubTask;

    #[test]
    fn report_usage() {
        let mut rng = rand::thread_rng();
        let mut provider = LinearUsageInflationProvider::new(0.1, 0.5, 1.0);
        let subtask = SubTask::new(100.0, 100.0);
        assert_almost_eq!(50.0, provider.report_usage(&mut rng, &subtask, 1.0), 1e-3);

        provider.num_subtasks_computed = 1;
        assert_almost_eq!(51.0, provider.report_usage(&mut rng, &subtask, 1.0), 1e-3);

        provider.num_subtasks_computed = 50;
        assert_almost_eq!(100.0, provider.report_usage(&mut rng, &subtask, 1.0), 1e-3);

        provider.num_subtasks_computed = 51;
        assert_almost_eq!(100.0, provider.report_usage(&mut rng, &subtask, 1.0), 1e-3);

        provider.num_subtasks_computed = 100;
        assert_almost_eq!(100.0, provider.report_usage(&mut rng, &subtask, 1.0), 1e-3);
    }
}
