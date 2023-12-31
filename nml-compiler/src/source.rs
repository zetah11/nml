use std::ops::{Add, AddAssign};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Identifies a particular source.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceId(usize);

impl SourceId {
    pub fn span(&self, start: usize, end: usize) -> Span {
        Span {
            source: *self,
            start,
            end,
        }
    }
}

impl SourceId {
    #[cfg(test)]
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

/// Identifies some portion of the source text.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Span {
    pub source: SourceId,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn length(&self) -> usize {
        self.end - self.start
    }
}

impl Add for Span {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        debug_assert_eq!(
            self.source, rhs.source,
            "only spans from the same source can be combined"
        );
        Self {
            source: self.source,
            start: self.start.min(rhs.start),
            end: self.end.max(rhs.end),
        }
    }
}

impl AddAssign for Span {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

#[derive(Debug)]
pub struct Source {
    pub id: SourceId,
    pub content: String,
}

/// Stores individual source files.
#[derive(Debug, Default)]
pub struct Sources {
    counter: AtomicUsize,
}

impl Sources {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&self, source: impl Into<String>) -> Source {
        let id = SourceId(self.counter.fetch_add(1, Ordering::SeqCst));
        Source {
            id,
            content: source.into(),
        }
    }
}
