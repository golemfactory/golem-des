use gd_world::prelude::*;
use gd_world::requestor::DefenceMechanismType;
use serde_derive::Deserialize;

use super::{Generator, ProviderBehaviour};

#[derive(Debug, Deserialize)]
pub struct RequestorSpec {
    #[serde(default)]
    id: Id,
    max_price: f64,
    budget_factor: f64,
    tasks: Vec<TaskSpec>,
    #[serde(default)]
    repeating: bool,
}

impl RequestorSpec {
    pub fn into_requestor<'a, Rng>(
        &self,
        rng: &'a mut Rng,
        defence_mechanism_type: DefenceMechanismType,
    ) -> Requestor
    where
        Rng: rand::Rng,
    {
        let mut requestor = Requestor::with_id(
            self.id,
            self.max_price,
            self.budget_factor,
            self.repeating,
            defence_mechanism_type,
        );
        requestor.append_tasks(
            self.tasks
                .iter()
                .map(|t| t.into_task(rng, self.max_price, self.budget_factor)),
        );

        requestor
    }
}

#[derive(Debug, Deserialize)]
pub struct TaskSpec {
    subtask_count: usize,
    nominal_usage: Generator,
}

impl TaskSpec {
    pub fn into_task<'a, Rng>(&self, rng: &'a mut Rng, max_price: f64, budget_factor: f64) -> Task
    where
        Rng: rand::Rng,
    {
        Task::new(self.subtask_count, || {
            let nominal_usage = self.nominal_usage.sample(rng);
            let budget = max_price * budget_factor * nominal_usage;

            SubTask::new(nominal_usage, budget)
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct ProviderSpec {
    #[serde(default)]
    id: Id,
    min_price: f64,
    usage_factor: f64,
    #[serde(default)]
    behaviour: ProviderBehaviour,
}

impl ProviderSpec {
    pub fn into_provider(&self) -> Box<dyn Provider> {
        match self.behaviour {
            ProviderBehaviour::UndercutBudget(epsilon) => {
                Box::new(UndercutBudgetProvider::with_id(
                    self.id,
                    self.min_price,
                    self.usage_factor,
                    epsilon,
                ))
            }
            ProviderBehaviour::LinearUsageInflation(factor) => {
                Box::new(LinearUsageInflationProvider::with_id(
                    self.id,
                    self.min_price,
                    self.usage_factor,
                    factor,
                ))
            }
            _ => Box::new(RegularProvider::with_id(
                self.id,
                self.min_price,
                self.usage_factor,
            )),
        }
    }
}
