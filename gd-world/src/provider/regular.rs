use std::fmt;
use std::marker::PhantomData;

use gd_world_derive::DerefProviderCommon;
use rand::distributions::Normal;
use rand::prelude::*;

use super::*;
use crate::task::SubTask;

#[derive(Debug, DerefProviderCommon)]
pub struct RegularProvider<Rng>
where
    Rng: rand::Rng + 'static,
{
    common: ProviderCommon,
    phantom: PhantomData<Rng>,
}

impl<Rng> RegularProvider<Rng>
where
    Rng: rand::Rng + 'static,
{
    const USAGE_JITTER: f64 = 0.05;

    pub fn new(min_price: f64, usage_factor: f64) -> Self {
        Self::with_id(Id::new(), min_price, usage_factor)
    }

    pub fn with_id(id: Id, min_price: f64, usage_factor: f64) -> Self {
        Self {
            common: ProviderCommon::new(id, min_price, usage_factor),
            phantom: PhantomData,
        }
    }
}

impl<Rng> fmt::Display for RegularProvider<Rng>
where
    Rng: rand::Rng + 'static,
{
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

impl<Rng> Provider for RegularProvider<Rng>
where
    Rng: rand::Rng + 'static,
{
    type Rng = Rng;

    fn report_usage(&self, rng: &mut Self::Rng, subtask: &SubTask, _bid: f64) -> f64 {
        let usage = subtask.nominal_usage * self.usage_factor();
        usage * (1.0 + Normal::new(0.0, Self::USAGE_JITTER).sample(rng))
    }

    fn into_stats(self: Box<Self>, run_id: u64) -> Stats {
        Stats {
            run_id,
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
        let mut provider = RegularProvider::new(0.1, 0.25);
        let subtask = SubTask::new(100.0, 100.0);

        assert_almost_eq!(
            provider.report_usage(&mut rng, &subtask, 0.1).round(),
            25.0,
            1e-6
        );
        assert_almost_eq!(
            provider.report_usage(&mut rng, &subtask, 0.5).round(),
            25.0,
            1e-6
        );
        assert_almost_eq!(
            provider.report_usage(&mut rng, &subtask, 1.0).round(),
            25.0,
            1e-6
        );

        provider.usage_factor = 0.75;

        assert_almost_eq!(
            provider.report_usage(&mut rng, &subtask, 0.1).round(),
            75.0,
            1e-6
        );
        assert_almost_eq!(
            provider.report_usage(&mut rng, &subtask, 0.5).round(),
            75.0,
            1e-6
        );
        assert_almost_eq!(
            provider.report_usage(&mut rng, &subtask, 1.0).round(),
            75.0,
            1e-6
        );
    }
}
