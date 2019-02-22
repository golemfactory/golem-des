use std::borrow::Borrow;
use std::f64;

use rgsl::randist::t_distribution;
use rgsl::statistics::{mean, sd_m};

pub trait ConfidenceIntervalForMean<T> {
    fn confidence_interval_for_mean(&self, confidence: f64) -> f64;
}

impl ConfidenceIntervalForMean<f64> for [f64] {
    fn confidence_interval_for_mean(&self, confidence: f64) -> f64 {
        IterStats::confidence_interval_for_mean(self, confidence)
    }
}

pub trait IterStats<T> {
    fn confidence_interval_for_mean(self, confidence: f64) -> f64;
}

impl<T> IterStats<f64> for T
where
    T: IntoIterator,
    T::Item: Borrow<f64>,
{
    fn confidence_interval_for_mean(self, confidence: f64) -> f64 {
        let data: Vec<f64> = self.into_iter().map(|x| *x.borrow()).collect();

        if data.is_empty() {
            return f64::NAN;
        }

        let n = data.len();
        let mean_data = mean(&data, 1, n);
        let sd_data = sd_m(&data, 1, n, mean_data);
        let alpha = (1.0 - confidence) / 2.0;
        let t = t_distribution::tdist_Qinv(alpha, n as f64 - 1.0);

        t * sd_data / (n as f64).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::f64;

    use statrs::assert_almost_eq;

    #[test]
    fn confidence_interval_for_mean() {
        let values = &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let ci = values.confidence_interval_for_mean(0.95);
        assert_almost_eq!(1.963, ci, 1e-3);

        let ci = values.confidence_interval_for_mean(0.99);
        assert_almost_eq!(3.079, ci, 1e-3);

        let values = &[1.0, f64::NAN, 3.0, 4.0, 5.0, 6.0];
        let ci = values.confidence_interval_for_mean(0.99);
        assert!(ci.is_nan());

        let values: &[f64] = &[];
        let ci = ConfidenceIntervalForMean::confidence_interval_for_mean(values, 0.99);
        assert!(ci.is_nan());
    }
}
