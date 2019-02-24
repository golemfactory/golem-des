use std::cmp::Ordering;
use std::fmt;
use std::ops::Add;
use std::slice::{Iter, IterMut};

pub trait Partitionable {
    type Item: Add<Output = Self::Item> + Copy;

    fn item(&self) -> Self::Item;
}

#[derive(Debug)]
pub struct Partition<T, P>
where
    T: PartialOrd + Add<Output = T> + Copy + fmt::Debug,
    P: fmt::Debug,
{
    lower: Option<T>,
    upper: Option<T>,
    values: Vec<P>,
}

impl<T, P> Partition<T, P>
where
    T: PartialOrd + Add<Output = T> + Copy + fmt::Debug,
    P: fmt::Debug,
{
    fn new(lower: Option<T>) -> Self {
        Self {
            lower,
            upper: None,
            values: Vec::new(),
        }
    }

    fn matches<F>(&self, value: &P, into: F) -> bool
    where
        F: Fn(&P) -> T,
    {
        let a = into(value);

        match (self.lower, self.upper) {
            (None, None) => true,
            (None, Some(upper)) => a < upper,
            (Some(lower), None) => lower <= a,
            (Some(lower), Some(upper)) => lower <= a && a < upper,
        }
    }

    fn insert(&mut self, value: P) {
        self.values.push(value)
    }

    pub fn boundaries(&self) -> (Option<T>, Option<T>) {
        (self.lower, self.upper)
    }

    pub fn iter(&self) -> Iter<P> {
        self.values.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<P> {
        self.values.iter_mut()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl<T, P> IntoIterator for Partition<T, P>
where
    T: PartialOrd + Add<Output = T> + Copy + fmt::Debug,
    P: fmt::Debug,
{
    type Item = P;
    type IntoIter = ::std::vec::IntoIter<P>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

pub fn partition_by<T, P, F>(mut values: Vec<P>, boundaries: &[T], into: F) -> Vec<Partition<T, P>>
where
    T: PartialOrd + Add<Output = T> + Copy + fmt::Debug,
    P: fmt::Debug,
    F: Fn(&P) -> T,
{
    if values.is_empty() {
        return Vec::new();
    }

    values.sort_unstable_by(|a, b| {
        let a = into(a);
        let b = into(b);

        match a.partial_cmp(&b) {
            None => Ordering::Equal,
            Some(x) => x,
        }
    });

    let mut partitions: Vec<Partition<T, P>> = Vec::new();
    let mut partition: Partition<T, P> = Partition::new(None);

    for &boundary in boundaries {
        partition.upper = Some(boundary);
        partitions.push(partition);
        partition = Partition::new(Some(boundary));
    }

    partitions.push(partition);

    let mut iter = partitions.iter_mut();
    let mut part = iter.next().unwrap();

    for value in values {
        while !part.matches(&value, &into) {
            part = iter.next().unwrap();
        }

        part.insert(value);
    }

    partitions
}

pub fn partition<T, P>(values: Vec<P>, boundaries: &[T]) -> Vec<Partition<T, P>>
where
    T: PartialOrd + Add<Output = T> + Copy + fmt::Debug,
    P: Partitionable<Item = T> + fmt::Debug,
{
    partition_by(values, boundaries, Partitionable::item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct TestValue(f64);

    impl Partitionable for TestValue {
        type Item = f64;

        fn item(&self) -> f64 {
            self.0
        }
    }

    #[test]
    fn matches() {
        let into = |tv: &TestValue| tv.item();

        let partition = Partition::new(None);

        assert!(partition.matches(&TestValue(0.5), &into));
        assert!(partition.matches(&TestValue(-0.5), &into));
        assert!(partition.matches(&TestValue(1.5), &into));

        let mut partition = Partition::new(None);
        partition.upper = Some(1.0);

        assert!(partition.matches(&TestValue(0.5), &into));
        assert!(partition.matches(&TestValue(-0.5), &into));
        assert!(!partition.matches(&TestValue(1.5), &into));

        let partition = Partition::new(Some(0.0));

        assert!(partition.matches(&TestValue(0.5), &into));
        assert!(!partition.matches(&TestValue(-0.5), &into));
        assert!(partition.matches(&TestValue(1.5), &into));

        let mut partition = Partition::new(Some(0.0));
        partition.upper = Some(1.0);

        assert!(partition.matches(&TestValue(0.5), &into));
        assert!(!partition.matches(&TestValue(-0.5), &into));
        assert!(!partition.matches(&TestValue(1.5), &into));
    }

    #[test]
    fn degenerate_partition() {
        let partitions = partition(Vec::<TestValue>::new(), &[0.0, 0.25]);
        assert!(partitions.is_empty());

        let partitions = partition(Vec::<TestValue>::new(), &[]);
        assert!(partitions.is_empty());

        let values = vec![TestValue(0.0)];
        let partitions = partition(values, &[]);
        assert_eq!(partitions.len(), 1);

        let part = &partitions[0];
        assert_eq!(part.lower, None);
        assert_eq!(part.upper, None);
        assert_eq!(part.values, vec![TestValue(0.0)]);
    }

    #[test]
    fn valid_partition() {
        let values = vec![
            TestValue(0.15),
            TestValue(0.45),
            TestValue(0.1),
            TestValue(1.1),
            TestValue(11.0),
            TestValue(0.25),
        ];

        let partitions = partition(values, &[0.0, 0.25, 0.5, 1.0, 10.0]);
        assert_eq!(partitions.len(), 6);

        let part = &partitions[0];
        assert_eq!(part.lower, None);
        assert_eq!(part.upper, Some(0.0));
        assert_eq!(part.values, vec![]);

        let part = &partitions[1];
        assert_eq!(part.lower, Some(0.0));
        assert_eq!(part.upper, Some(0.25));
        assert_eq!(part.values, vec![TestValue(0.1), TestValue(0.15)]);

        let part = &partitions[2];
        assert_eq!(part.lower, Some(0.25));
        assert_eq!(part.upper, Some(0.5));
        assert_eq!(part.values, vec![TestValue(0.25), TestValue(0.45)]);

        let part = &partitions[3];
        assert_eq!(part.lower, Some(0.5));
        assert_eq!(part.upper, Some(1.0));
        assert_eq!(part.values, vec![]);

        let part = &partitions[4];
        assert_eq!(part.lower, Some(1.0));
        assert_eq!(part.upper, Some(10.0));
        assert_eq!(part.values, vec![TestValue(1.1)]);

        let part = &partitions[5];
        assert_eq!(part.lower, Some(10.0));
        assert_eq!(part.upper, None);
        assert_eq!(part.values, vec![TestValue(11.0)]);
    }
}
