use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fmt::Debug;

#[derive(Debug)]
struct Event<Payload: Debug> {
    time: f64,
    payload: Payload,
}

impl<Payload> Eq for Event<Payload> where Payload: Debug {}

impl<Payload> PartialEq for Event<Payload>
where
    Payload: Debug,
{
    fn eq(&self, other: &Event<Payload>) -> bool {
        self.time == other.time
    }
}

impl<Payload> PartialOrd for Event<Payload>
where
    Payload: Debug,
{
    fn partial_cmp(&self, other: &Event<Payload>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Payload> Ord for Event<Payload>
where
    Payload: Debug,
{
    fn cmp(&self, other: &Event<Payload>) -> Ordering {
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
pub struct Engine<Payload: Debug> {
    now: f64,
    events: BinaryHeap<Event<Payload>>,
}

impl<Payload> Engine<Payload>
where
    Payload: Debug,
{
    pub fn new() -> Engine<Payload> {
        Engine {
            now: 0.0,
            events: BinaryHeap::new(),
        }
    }

    pub fn schedule(&mut self, after: f64, payload: Payload) {
        self.events.push(Event {
            time: self.now + after,
            payload: payload,
        });
    }

    pub fn next(&mut self) -> Option<Payload> {
        self.events.pop().map(|e| {
            self.now = e.time;
            e.payload
        })
    }

    pub fn now(&self) -> f64 {
        self.now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn event_queue() {
        let mut engine = Engine::new();
        assert!(engine.events.is_empty());
        assert_eq!(engine.now(), 0.0);

        engine.schedule(2.0, 3);
        engine.schedule(1.0, 2);
        engine.schedule(0.5, 1);

        assert_eq!(engine.next(), Some(1));
        assert_eq!(engine.now(), 0.5);

        assert_eq!(engine.next(), Some(2));
        assert_eq!(engine.now(), 1.0);

        assert_eq!(engine.next(), Some(3));
        assert_eq!(engine.now(), 2.0);

        assert_eq!(engine.next(), None);
    }
}
