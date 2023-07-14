use std::cell::Cell;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Level(Cell<usize>);

impl Level {
    pub fn new(level: usize) -> Self {
        Self(Cell::new(level))
    }

    pub fn can_generalize(&self, current: usize) -> bool {
        self.0.get() > current
    }

    pub fn min(&self, other: &Self) -> usize {
        self.0.get().min(other.0.get())
    }

    /// Set `self` to be the minimum level of `self` and `other`
    pub fn set_min(&self, other: &Self) {
        self.0.set(self.0.get().min(other.0.get()))
    }
}
