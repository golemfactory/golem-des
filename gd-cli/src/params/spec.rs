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
    pub fn as_requestor<'a, Rng>(
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
            defence_mechanism_type,
        );
        requestor.task_queue_mut().append(
            self.tasks
                .iter()
                .map(|t| t.as_task(rng, self.max_price, self.budget_factor)),
        );
        requestor.task_queue_mut().repeating = self.repeating;

        requestor
    }
}

#[derive(Debug, Deserialize)]
pub struct TaskSpec {
    subtask_count: usize,
    nominal_usage: Generator,
}

impl TaskSpec {
    pub fn as_task<'a, Rng>(&self, rng: &'a mut Rng, max_price: f64, budget_factor: f64) -> Task
    where
        Rng: rand::Rng,
    {
        let mut task = Task::new();

        for _ in 0..self.subtask_count {
            let nominal_usage = self.nominal_usage.sample(rng);
            let budget = max_price * budget_factor * nominal_usage;

            task.push_pending(SubTask::new(nominal_usage, budget));
        }

        task
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
    pub fn as_provider<Rng>(&self) -> Box<dyn Provider<Rng = Rng>>
    where
        Rng: rand::Rng + 'static,
    {
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
