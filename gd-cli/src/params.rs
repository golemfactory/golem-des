mod source;
mod spec;

pub use self::source::*;
pub use self::spec::*;

use rand::distributions::{Exp, LogNormal, Normal, Uniform};
use rand::prelude::*;
use serde_derive::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Generator {
    Fixed(f64),
    Choice(Vec<f64>),
    Uniform(f64, f64),
    LogNormal(f64, f64),
    Normal(f64, f64),
    Exp(f64),
}

impl Generator {
    pub fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> f64 {
        match self {
            &Generator::Fixed(value) => value,
            &Generator::Choice(ref values) => *values.choose(rng).unwrap_or(&0.0),
            &Generator::Uniform(min, max) => Uniform::new(min, max).sample(rng),
            &Generator::LogNormal(mean, std) => LogNormal::new(mean, std).sample(rng),
            &Generator::Normal(mean, std) => Normal::new(mean, std).sample(rng),
            &Generator::Exp(mean) => Exp::new(mean).sample(rng),
        }
    }
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderBehaviour {
    Regular,
    UndercutBudget(f64),
    LinearUsageInflation(f64),
}

impl Default for ProviderBehaviour {
    fn default() -> Self {
        ProviderBehaviour::Regular
    }
}

#[derive(Debug, Deserialize)]
pub struct SimulationParams {
    pub duration: f64,
    pub seed: Option<u64>,
    pub requestors: Option<Vec<RequestorSpec>>,
    pub requestor_sources: Option<Vec<RequestorSource>>,
    pub providers: Option<Vec<ProviderSpec>>,
    pub provider_sources: Option<Vec<ProviderSource>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_test::{assert_de_tokens, Token};

    #[test]
    fn test_deserialize_provider_behaviour() {
        assert_de_tokens(
            &ProviderBehaviour::Regular,
            &[
                Token::Enum {
                    name: "ProviderBehaviour",
                },
                Token::Str("regular"),
                Token::Unit,
            ],
        );

        assert_de_tokens(
            &ProviderBehaviour::UndercutBudget(0.01),
            &[
                Token::Enum {
                    name: "ProviderBehaviour",
                },
                Token::Str("undercut_budget"),
                Token::F64(0.01),
            ],
        );

        assert_de_tokens(
            &ProviderBehaviour::LinearUsageInflation(1.5),
            &[
                Token::Enum {
                    name: "ProviderBehaviour",
                },
                Token::Str("linear_usage_inflation"),
                Token::F64(1.5),
            ],
        );
    }

    #[test]
    fn test_deserialize_fixed() {
        assert_de_tokens(
            &Generator::Fixed(1.0),
            &[
                Token::Enum { name: "Generator" },
                Token::Str("fixed"),
                Token::F64(1.0),
            ],
        );
    }

    #[test]
    fn test_deserialize_choice() {
        assert_de_tokens(
            &Generator::Choice(vec![0.5, 1.0, 2.0]),
            &[
                Token::Enum { name: "Generator" },
                Token::Str("choice"),
                Token::Seq { len: Some(3) },
                Token::F64(0.5),
                Token::F64(1.0),
                Token::F64(2.0),
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn test_deserialize_uniform() {
        assert_de_tokens(
            &Generator::Uniform(1.0, 2.0),
            &[
                Token::Enum { name: "Generator" },
                Token::Str("uniform"),
                Token::Seq { len: Some(2) },
                Token::F64(1.0),
                Token::F64(2.0),
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn test_deserialize_lognormal() {
        assert_de_tokens(
            &Generator::LogNormal(0.0, 1.0),
            &[
                Token::Enum { name: "Generator" },
                Token::Str("lognormal"),
                Token::Seq { len: Some(2) },
                Token::F64(0.0),
                Token::F64(1.0),
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn test_deserialize_normal() {
        assert_de_tokens(
            &Generator::Normal(0.0, 0.5),
            &[
                Token::Enum { name: "Generator" },
                Token::Str("normal"),
                Token::Seq { len: Some(2) },
                Token::F64(0.0),
                Token::F64(0.5),
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn test_deserialize_exp() {
        assert_de_tokens(
            &Generator::Exp(1.0),
            &[
                Token::Enum { name: "Generator" },
                Token::Str("exp"),
                Token::F64(1.0),
            ],
        );
    }
}
