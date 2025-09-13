use core::hash::Hash;
use core::marker::PhantomData;
use core::ops::RangeBounds;
use std::collections::hash_set::Iter;

use rustc_hash::FxHashSet;

#[derive(Debug, Default, Clone)]
pub struct RangeSet<R: RangeBounds<Idx>, Idx> {
    data: FxHashSet<R>,
    phantom: PhantomData<Idx>,
}

impl<R, Idx> RangeSet<R, Idx>
where
    R: RangeBounds<Idx>,
{
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        let data = FxHashSet::default();
        let phantom = PhantomData;
        Self { data, phantom }
    }

    #[inline]
    #[must_use]
    pub fn iter(&self) -> Iter<'_, R> {
        self.data.iter()
    }

    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }
}

impl<R, Idx> RangeSet<R, Idx>
where
    R: RangeBounds<Idx>,
    Idx: PartialOrd,
{
    #[inline]
    #[must_use]
    pub fn contains(&self, value: &Idx) -> bool {
        self.data.iter().any(|range| range.contains(value))
    }
}

impl<R, Idx> RangeSet<R, Idx>
where
    R: RangeBounds<Idx> + Eq + Hash,
{
    #[inline]
    pub fn insert(&mut self, value: R) {
        self.data.insert(value);
    }
}

impl<R, Idx> PartialEq for RangeSet<R, Idx>
where
    R: RangeBounds<Idx> + Eq + Hash,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<R, Idx> IntoIterator for RangeSet<R, Idx>
where
    R: RangeBounds<Idx>,
{
    type Item = R;
    type IntoIter = <FxHashSet<R> as IntoIterator>::IntoIter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl<'a, R, Idx> IntoIterator for &'a RangeSet<R, Idx>
where
    R: RangeBounds<Idx>,
{
    type Item = &'a R;
    type IntoIter = Iter<'a, R>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<R, Idx, const N: usize> From<[R; N]> for RangeSet<R, Idx>
where
    R: RangeBounds<Idx> + Eq + Hash,
{
    #[inline]
    fn from(value: [R; N]) -> Self {
        let data = FxHashSet::from_iter(value);
        let phantom = PhantomData;
        Self { data, phantom }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::{assert_eq, assert_ne};

    use super::*;

    #[test]
    fn insert() {
        let mut set = RangeSet::new();
        assert!(!set.data.contains(&(0..10)));
        set.insert(0..10);
        assert!(set.data.contains(&(0..10)));
    }

    #[test]
    fn len() {
        let mut set = RangeSet::new();
        assert_eq!(set.len(), 0);
        set.insert(0..10);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn iter() {
        let ranges = [0..10, 20..30, 25..35];
        let set = RangeSet::from(ranges.clone());
        let actual: FxHashSet<_> = set.iter().collect();
        let expected: FxHashSet<_> = ranges.iter().collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn into_iter() {
        let ranges = [0..10, 20..30, 25..35];
        let set = RangeSet::from(ranges.clone());
        let actual: FxHashSet<_> = set.into_iter().collect();
        let expected: FxHashSet<_> = ranges.into_iter().collect();
        assert_eq!(actual, expected);
    }

    #[test]
    #[allow(clippy::into_iter_on_ref)]
    fn into_iter_ref() {
        let ranges = &[0..10, 20..30, 25..35];
        let set = &RangeSet::from(ranges.clone());
        let actual: FxHashSet<_> = set.into_iter().collect();
        let expected: FxHashSet<_> = ranges.into_iter().collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn is_empty() {
        let mut set = RangeSet::new();
        assert!(set.is_empty());
        set.insert(0..10);
        assert!(!set.is_empty());
    }

    #[test]
    fn contains_range() {
        let set = RangeSet::from([0..10, 20..30, 25..35]);
        assert!(set.contains(&0));
        assert!(!set.contains(&10));
        assert!(set.contains(&20));
        assert!(set.contains(&30));
        assert!(!set.contains(&35));
    }

    #[test]
    fn contains_range_inclusive() {
        let set = RangeSet::from([0..=10, 20..=30, 25..=35]);
        assert!(set.contains(&0));
        assert!(set.contains(&10));
        assert!(set.contains(&20));
        assert!(set.contains(&30));
        assert!(set.contains(&35));
    }

    #[test]
    fn partial_eq_true() {
        let ranges = [0..10, 20..30, 25..35];
        let set0 = RangeSet::from(ranges.clone());
        let set1 = RangeSet::from(ranges);
        assert_eq!(set0, set1);
    }

    #[test]
    fn partial_eq_false() {
        let ranges0 = [0..10, 20..30, 25..35];
        let ranges1 = [0..10, 20..30, 35..45];
        let set0 = RangeSet::from(ranges0);
        let set1 = RangeSet::from(ranges1);
        assert_ne!(set0, set1);
    }

    #[test]
    fn from_array() {
        let ranges = [0..10, 20..30, 25..35];
        let set = RangeSet::from(ranges.clone());
        let expected: FxHashSet<_> = ranges.iter().map(ToOwned::to_owned).collect();
        assert_eq!(set.data, expected);
    }
}
