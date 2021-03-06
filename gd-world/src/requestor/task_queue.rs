use std::collections::VecDeque;

use crate::task::Task;

#[derive(Debug)]
pub struct TaskQueue {
    buffer: VecDeque<Task>,
    pub repeating: bool,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, task: Task) {
        self.buffer.push_back(task)
    }

    pub fn pop(&mut self) -> Option<Task> {
        self.buffer.pop_front().map(|task| {
            if self.repeating {
                self.buffer.push_back(task.clone());
            }

            task
        })
    }

    pub fn append<It: IntoIterator<Item = Task>>(&mut self, tasks: It) {
        for task in tasks {
            self.push(task)
        }
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self {
            buffer: VecDeque::new(),
            repeating: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pop_repeating() {
        let mut task_queue = TaskQueue::new();

        assert!(task_queue.repeating);

        let task = Task::new();
        task_queue.push(task.clone());

        assert_eq!(task_queue.pop(), Some(task.clone()));
        assert_eq!(task_queue.pop(), Some(task));
    }

    #[test]
    fn pop_nonrepeating() {
        let mut task_queue = TaskQueue::new();
        task_queue.repeating = false;

        let task = Task::new();
        task_queue.push(task.clone());

        assert_eq!(task_queue.pop(), Some(task));
        assert_eq!(task_queue.pop(), None);
    }
}
