mod linear_usage_inflation;
mod regular;
mod undercut_budget;

pub use self::linear_usage_inflation::LinearUsageInflationProvider;
pub use self::regular::RegularProvider;
pub use self::undercut_budget::UndercutBudgetProvider;

use std::any::Any;
use std::fmt;
use std::ops::{Deref, DerefMut};

use gd_engine::Engine;
use log::debug;
use serde_derive::{Deserialize, Serialize};

use crate::id::Id;
use crate::task::SubTask;
use crate::world::Event;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Behaviour {
    Regular,
    LinearUsageInflation,
    UndercutBudget,
}

impl fmt::Display for Behaviour {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Behaviour::Regular => "Regular",
                Behaviour::LinearUsageInflation => "Linear usage inflation",
                Behaviour::UndercutBudget => "Undercut budget",
            }
        )
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub run_id: u64,
    pub behaviour: Behaviour,
    pub min_price: f64,
    pub usage_factor: f64,
    pub profit_margin: f64,
    pub price: f64,
    pub revenue: f64,
    pub num_subtasks_assigned: usize,
    pub num_subtasks_computed: usize,
    pub num_subtasks_cancelled: usize,
}

pub trait Provider: fmt::Debug + fmt::Display {
    fn report_usage(&self, subtask: &SubTask, bid: f64) -> f64;
    fn into_stats(self: Box<Self>, run_id: u64) -> Stats;

    fn as_provider_common(&self) -> &ProviderCommon;
    fn as_provider_common_mut(&mut self) -> &mut ProviderCommon;

    fn as_any(&self) -> &dyn Any;
}

impl Deref for Provider {
    type Target = ProviderCommon;

    fn deref(&self) -> &ProviderCommon {
        self.as_provider_common()
    }
}

impl DerefMut for Provider {
    fn deref_mut(&mut self) -> &mut ProviderCommon {
        self.as_provider_common_mut()
    }
}

#[derive(Debug, PartialEq)]
enum State {
    Idle,
    Busy,
}

#[derive(Debug)]
pub struct ProviderCommon {
    id: Id,
    min_price: f64,
    usage_factor: f64,
    state: State,
    profit_margin: f64,
    last_checkpoint: f64,
    revenue: f64,
    num_subtasks_assigned: usize,
    num_subtasks_computed: usize,
    num_subtasks_cancelled: usize,
}

impl ProviderCommon {
    const ALPHA: f64 = 1e-5;
    const BETA: f64 = 1e-5;

    fn new(id: Id, min_price: f64, usage_factor: f64) -> ProviderCommon {
        ProviderCommon {
            id: id,
            min_price: min_price,
            usage_factor: usage_factor,
            state: State::Idle,
            profit_margin: 1.0,
            last_checkpoint: 0.0,
            revenue: 0.0,
            num_subtasks_assigned: 0,
            num_subtasks_computed: 0,
            num_subtasks_cancelled: 0,
        }
    }

    pub fn id(&self) -> &Id {
        &self.id
    }

    pub fn usage_factor(&self) -> f64 {
        self.usage_factor
    }

    fn price(&self) -> f64 {
        (1.0 + self.profit_margin) * self.min_price
    }

    fn increase_profit_margin(&mut self, duration: f64) {
        let old_profit_margin = self.profit_margin;
        self.profit_margin *= (ProviderCommon::BETA * duration).exp();

        debug!(
            "P{}:increasing profit margin: {} => {}, duration = {}",
            self.id, old_profit_margin, self.profit_margin, duration
        );
    }

    fn decrease_profit_margin(&mut self, duration: f64) {
        let old_profit_margin = self.profit_margin;
        self.profit_margin *= (-ProviderCommon::ALPHA * duration).exp();

        debug!(
            "P{}:decreasing profit margin: {} => {}, duration = {}",
            self.id, old_profit_margin, self.profit_margin, duration
        );
    }

    pub fn send_benchmark(&self) -> f64 {
        self.usage_factor
    }

    pub fn send_offer(&mut self) -> Option<f64> {
        match self.state {
            State::Idle => Some(self.price()),
            _ => None,
        }
    }

    pub fn receive_subtask<Rng>(
        &mut self,
        engine: &mut Engine<Event>,
        _rng: &mut Rng,
        subtask: &SubTask,
        requestor_id: &Id,
        bid: f64,
    ) where
        Rng: rand::Rng,
    {
        debug!("P{}:received {} from R{}", self.id, subtask, requestor_id);

        self.state = State::Busy;
        self.num_subtasks_assigned += 1;

        self.decrease_profit_margin(engine.now() - self.last_checkpoint);
        self.last_checkpoint = engine.now();

        let expected_usage = subtask.nominal_usage * self.usage_factor;
        if expected_usage * bid > subtask.budget {
            // schedule budget exceeded event
            engine.schedule(
                subtask.budget / bid,
                Event::SubTaskBudgetExceeded(*subtask, *requestor_id, self.id),
            );
        } else {
            // schedule subtask computed event
            engine.schedule(
                expected_usage,
                Event::SubTaskComputed(*subtask, *requestor_id, self.id, bid),
            );
        }
    }

    pub fn finish_computing(&mut self, now: f64, subtask: &SubTask, requestor_id: &Id) {
        debug!(
            "P{}:finished computing {} of R{}",
            self.id, subtask, requestor_id,
        );

        self.state = State::Idle;
        self.num_subtasks_computed += 1;

        self.increase_profit_margin(now - self.last_checkpoint);
        self.last_checkpoint = now;
    }

    pub fn receive_payment(&mut self, subtask: &SubTask, requestor_id: &Id, payment: Option<f64>) {
        match payment {
            Some(payment) => {
                debug!(
                    "P{}:received {} from R{} for {}",
                    self.id, payment, requestor_id, subtask
                );
                self.revenue += payment;
            }
            None => debug!(
                "P{}:no payment received from R{} for {}",
                self.id, requestor_id, subtask
            ),
        }
    }

    pub fn cancel_computing(&mut self, now: f64, subtask: &SubTask, requestor_id: &Id) {
        debug!(
            "P{}:budget exceeded for {} of R{}",
            self.id, subtask, requestor_id
        );

        self.state = State::Idle;
        self.num_subtasks_cancelled += 1;

        self.last_checkpoint = now;
    }
}

impl fmt::Display for ProviderCommon {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            r"Id:                             {},
            Min price:                      {},
            Usage factor:                   {},
            Profit margin:                  {},
            Price:                          {},
            Revenue:                        {},
            Number of subtasks assigned:    {},
            Number of subtasks cancelled:   {},
            Nunber of subtasks computed:    {}",
            self.id,
            self.min_price,
            self.usage_factor,
            self.profit_margin,
            self.price(),
            self.revenue,
            self.num_subtasks_assigned,
            self.num_subtasks_cancelled,
            self.num_subtasks_computed,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use statrs::assert_almost_eq;

    #[test]
    fn send_offer() {
        let mut provider = ProviderCommon::new(Id::new(), 1.0, 1.0);
        provider.state = State::Idle;

        assert_eq!(provider.send_offer(), Some(2.0));

        provider.state = State::Busy;

        assert_eq!(provider.send_offer(), None);
    }

    #[test]
    fn increase_profit_margin() {
        let mut provider = ProviderCommon::new(Id::new(), 1.0, 1.0);

        assert_almost_eq!(provider.profit_margin, 1.0, 1e-5);

        provider.increase_profit_margin(1000.0);

        assert_almost_eq!(provider.profit_margin, 1.01005, 1e-5);
    }

    #[test]
    fn decrease_profit_margin() {
        let mut provider = ProviderCommon::new(Id::new(), 1.0, 1.0);

        assert_almost_eq!(provider.profit_margin, 1.0, 1e-5);

        provider.decrease_profit_margin(1000.0);

        assert_almost_eq!(provider.profit_margin, 0.99004, 1e-5);

    }
}
