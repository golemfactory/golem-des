#![warn(clippy::all)]

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt;

#[derive(Debug)]
struct EventWrapper<Event>
where
    Event: fmt::Debug,
{
    time: f64,
    event: Event,
}

impl<Event> Eq for EventWrapper<Event> where Event: fmt::Debug {}

impl<Event> PartialEq for EventWrapper<Event>
where
    Event: fmt::Debug,
{
    fn eq(&self, other: &EventWrapper<Event>) -> bool {
        self.time == other.time
    }
}

impl<Event> PartialOrd for EventWrapper<Event>
where
    Event: fmt::Debug,
{
    fn partial_cmp(&self, other: &EventWrapper<Event>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Event> Ord for EventWrapper<Event>
where
    Event: fmt::Debug,
{
    fn cmp(&self, other: &EventWrapper<Event>) -> Ordering {
        if self.time < other.time {
            Ordering::Greater
        } else if self.time > other.time {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
}

#[derive(Debug)]
pub struct Engine<Event>
where
    Event: fmt::Debug,
{
    now: f64,
    events: BinaryHeap<EventWrapper<Event>>,
}

impl<Event> Engine<Event>
where
    Event: fmt::Debug,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn schedule(&mut self, after: f64, event: Event) {
        self.events.push(EventWrapper {
            time: self.now + after,
            event,
        });
    }

    pub fn pop(&mut self) -> Option<Event> {
        self.events.pop().map(|e| {
            self.now = e.time;
            e.event
        })
    }

    pub fn now(&self) -> f64 {
        self.now
    }
}

impl<Event> Default for Engine<Event>
where
    Event: fmt::Debug,
{
    fn default() -> Self {
        Self {
            now: 0.0,
            events: BinaryHeap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use statrs::assert_almost_eq;

    #[test]
    pub fn event_queue() {
        let mut engine = Engine::new();
        assert!(engine.events.is_empty());
        assert_almost_eq!(engine.now(), 0.0, 1e-6);

        engine.schedule(2.0, 3);
        engine.schedule(1.0, 2);
        engine.schedule(0.5, 1);

        assert_eq!(engine.pop(), Some(1));
        assert_almost_eq!(engine.now(), 0.5, 1e-6);

        assert_eq!(engine.pop(), Some(2));
        assert_almost_eq!(engine.now(), 1.0, 1e-6);

        assert_eq!(engine.pop(), Some(3));
        assert_almost_eq!(engine.now(), 2.0, 1e-6);

        assert_eq!(engine.pop(), None);
    }
}
