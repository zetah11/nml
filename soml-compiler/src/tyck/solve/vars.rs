use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Level(Rc<Cell<usize>>);

impl Level {
    pub fn new(level: usize) -> Self {
        Self(Rc::new(Cell::new(level)))
    }

    pub fn can_generalize(&self, current: usize) -> bool {
        self.0.get() > current
    }

    /// Set `self` to be the minimum level of `self` and `other`
    pub fn set_min(&self, other: &Self) {
        self.0.set(self.0.get().min(other.0.get()))
    }

    pub fn as_usize(&self) -> usize {
        self.0.get()
    }
}
