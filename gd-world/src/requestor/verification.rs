use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

type VerificationResult<T> = Option<(T, f64)>;

#[derive(Debug)]
pub struct VerificationMap<T>
where
    T: fmt::Debug + Hash + Eq,
{
    verification_factor: usize,
    map: HashMap<T, Vec<VerificationResult<T>>>,
}

impl<T> VerificationMap<T>
where
    T: fmt::Debug + Hash + Eq,
{
    const DEFAULT_FACTOR: usize = 2;

    pub fn new(factor: usize) -> VerificationMap<T> {
        if factor != Self::DEFAULT_FACTOR {
            unimplemented!();
        }

        VerificationMap {
            verification_factor: factor,
            map: HashMap::new(),
        }
    }

    pub fn insert_key(&mut self, key: T) {
        self.map
            .insert(key, Vec::with_capacity(Self::DEFAULT_FACTOR));
    }

    pub fn insert_verification(
        &mut self,
        key: &T,
        res: VerificationResult<T>,
    ) -> Option<Vec<(T, f64)>> {
        if !self.map.contains_key(key) {
            panic!("verification key not found");
        }

        self.map.get_mut(key).unwrap().push(res);

        if self.map.get(key).unwrap().len() == self.verification_factor {
            Some(
                self.map
                    .remove(key)
                    .unwrap()
                    .into_iter()
                    .filter_map(|v| v)
                    .collect(),
            )
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn test_constructor() {
        let mut _vmap = VerificationMap::<usize>::new(0);
        _vmap = VerificationMap::new(1);
        _vmap = VerificationMap::new(3);
    }

    #[test]
    fn test_insert_verification() {
        let mut vmap = VerificationMap::new(2);
        vmap.insert_key(0);

        assert_eq!(vmap.insert_verification(&0, Some((1, 1.0))), None);
        assert_eq!(
            vmap.insert_verification(&0, Some((2, 1.0))),
            Some(vec![(1, 1.0), (2, 1.0)])
        );
        assert_eq!(vmap.map.get(&0), None);

        vmap.insert_key(0);

        assert_eq!(vmap.insert_verification(&0, Some((1, 1.0))), None);
        assert_eq!(vmap.insert_verification(&0, None), Some(vec![(1, 1.0)]));
        assert_eq!(vmap.map.get(&0), None);

        vmap.insert_key(0);

        assert_eq!(vmap.insert_verification(&0, None), None);
        assert_eq!(
            vmap.insert_verification(&0, Some((2, 1.0))),
            Some(vec![(2, 1.0)])
        );
        assert_eq!(vmap.map.get(&0), None);

        vmap.insert_key(0);

        assert_eq!(vmap.insert_verification(&0, None), None);
        assert_eq!(vmap.insert_verification(&0, None), Some(vec![]));
        assert_eq!(vmap.map.get(&0), None);
    }
}
