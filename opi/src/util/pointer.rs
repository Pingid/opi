use std::marker::PhantomData;

pub struct Point<T> {
    _marker: PhantomData<T>,
    pub index: usize,
}

impl<T> Clone for Point<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Point<T> {}

impl<T> std::hash::Hash for Point<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}

impl<T> std::cmp::Ord for Point<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

impl<T> std::cmp::PartialOrd for Point<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> std::fmt::Debug for Point<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Idx({})", self.index)
    }
}

impl<T> Eq for Point<T> {}

impl<T> PartialEq<Point<T>> for Point<T> {
    fn eq(&self, other: &Point<T>) -> bool {
        self.index == other.index
    }
}
