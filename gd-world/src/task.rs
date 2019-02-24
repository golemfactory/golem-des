use std::collections::VecDeque;
use std::fmt;

use crate::id::Id;

pub use subtask::SubTask;

#[derive(Clone, Debug, Default)]
pub struct Task {
    id: Id,
    size: usize,
    pending: VecDeque<SubTask>,
    done: VecDeque<SubTask>,
}

impl Task {
    pub fn new() -> Self {
        Self::default()
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
        !self.pending.is_empty()
    }

    pub fn is_done(&self) -> bool {
        self.done.len() == self.size
    }
}

impl std::fmt::Display for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Task({}, {}, {}, {})",
            self.id,
            self.size,
            self.pending.len(),
            self.done.len(),
        )
    }
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub mod subtask {
    use super::*;

    #[derive(Debug, PartialEq)]
    pub enum Status {
        Pending,
        Cancelled,
        Done,
    }

    #[derive(Clone, Copy, Debug)]
    pub struct SubTask {
        id: Id,
        pub nominal_usage: f64,
        pub budget: f64,
    }

    impl SubTask {
        pub fn new(nominal_usage: f64, budget: f64) -> Self {
            Self {
                id: Id::new(),
                nominal_usage,
                budget,
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
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_states() {
        let mut task = Task::new();

        assert!(!task.is_pending());
        assert!(task.is_done());

        let subtask = SubTask::new(1.0, 1.0);
        task.push_pending(subtask);

        assert!(task.is_pending());
        assert!(!task.is_done());

        task.pop_pending();

        assert!(!task.is_pending());
        assert!(!task.is_done());

        task.push_done(subtask);

        assert!(!task.is_pending());
        assert!(task.is_done());
    }
}
