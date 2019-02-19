use std::collections::VecDeque;
use std::fmt;

use crate::id::Id;

#[derive(Clone, Debug, PartialEq)]
pub struct Task {
    size: usize,
    pending: VecDeque<SubTask>,
    done: VecDeque<SubTask>,
}

impl Task {
    pub fn new() -> Task {
        Task {
            size: 0,
            pending: VecDeque::new(),
            done: VecDeque::new(),
        }
    }

    pub fn push_pending(&mut self, subtask: SubTask) {
        self.size += 1;
        self.pending.push_back(subtask);
    }

    pub fn pop_pending(&mut self) -> Option<SubTask> {
        self.pending.pop_front()
    }

    pub fn push_done(&mut self, subtask: SubTask) {
        self.done.push_back(subtask)
    }

    pub fn is_pending(&self) -> bool {
        self.pending.len() > 0
    }

    pub fn is_done(&self) -> bool {
        self.done.len() == self.size
    }
}

impl std::fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Task({}, {}, {})",
            self.size,
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

    pub fn id(&self) -> &Id {
        &self.id
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

impl PartialEq for SubTask {
    fn eq(&self, other: &SubTask) -> bool {
        self.id == other.id
    }
}
