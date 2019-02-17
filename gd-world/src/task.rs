use std::collections::VecDeque;
use std::fmt;

use crate::id::Id;

#[derive(Debug, Clone)]
pub struct Task {
    pub subtask_count: usize,
    pending: VecDeque<SubTask>,
    done: VecDeque<SubTask>,
}

impl Task {
    pub fn new<F>(subtask_count: usize, mut generator: F) -> Task
    where
        F: FnMut() -> SubTask,
    {
        Task {
            subtask_count: subtask_count,
            pending: (0..subtask_count).map(|_| generator()).collect(),
            done: VecDeque::with_capacity(subtask_count),
        }
    }

    pub fn is_pending(&self) -> bool {
        self.pending.len() > 0
    }

    pub fn is_done(&self) -> bool {
        self.done.len() == self.subtask_count
    }

    pub fn pop_subtask(&mut self) -> Option<SubTask> {
        self.pending.pop_front()
    }

    pub fn push_subtask(&mut self, subtask: SubTask) {
        self.pending.push_back(subtask)
    }

    pub fn subtask_computed(&mut self, subtask: SubTask) {
        self.done.push_back(subtask)
    }
}

impl std::fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Task({}, {}, {})",
            self.subtask_count,
            self.pending.len(),
            self.done.len(),
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SubTask {
    id: Id,
    pub nominal_usage: f64,
    pub budget: f64,
}

impl SubTask {
    pub fn new(nominal_usage: f64, budget: f64) -> SubTask {
        SubTask {
            id: Id::new(),
            nominal_usage: nominal_usage,
            budget: budget,
        }
    }

    pub fn id(&self) -> Id {
        self.id
    }
}

impl std::fmt::Display for SubTask {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SubTask({}, {}, {})",
            self.id, self.nominal_usage, self.budget
        )
    }
}
