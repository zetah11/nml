use std::collections::HashMap;

use crate::frontend::source::{SourceId, Span};

/// Identifies a particular reported message.
#[must_use]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ErrorId(usize);

impl ErrorId {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

/// Stores reported errors.
#[derive(Clone, Debug, Default)]
pub struct Errors {
    errors: HashMap<ErrorId, Error>,
    counter: usize,

    num_errors: usize,
    num_warnings: usize,
    num_infos: usize,
}

impl Errors {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, error: Error) -> ErrorId {
        match error.severity {
            Severity::Error => self.num_errors += 1,
            Severity::Warning => self.num_warnings += 1,
            Severity::Info => self.num_infos += 1,
        };

        self.counter += 1;
        let id = ErrorId(self.counter);
        self.errors.insert(id, error);
        id
    }

    pub fn is_perfect(&self) -> bool {
        self.num_errors == 0 && self.num_warnings == 0
    }

    pub fn num_errors(&self) -> usize {
        self.num_errors
    }

    pub fn num_warnings(&self) -> usize {
        self.num_warnings
    }

    /// Get an iterator over every source id mentioned by any of the errors in this.
    /// May contain duplicates.
    pub fn sources(&self) -> impl Iterator<Item = SourceId> + '_ {
        self.errors.values().flat_map(Error::sources)
    }

    /// Drain this error store of its errors.
    pub fn drain(&mut self) -> impl Iterator<Item = (ErrorId, Error)> + '_ {
        self.num_errors = 0;
        self.num_warnings = 0;
        self.num_infos = 0;
        self.errors.drain()
    }
}

#[derive(Clone, Debug)]
pub struct Error {
    pub ty: ErrorType,
    pub severity: Severity,
    pub at: Span,
    pub title: String,
    pub labels: Vec<(String, Span)>,
    pub notes: Vec<(String, NoteType)>,
}

impl Error {
    pub fn new(ty: ErrorType, severity: Severity, at: Span, title: impl Into<String>) -> Self {
        Self {
            ty,
            severity,
            at,
            title: title.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn with_label(mut self, at: Span, message: impl Into<String>) -> Self {
        self.labels.push((message.into(), at));
        self
    }

    pub fn with_help(mut self, message: impl Into<String>) -> Self {
        self.notes.push((message.into(), NoteType::Help));
        self
    }

    pub fn with_note(mut self, message: impl Into<String>) -> Self {
        self.notes.push((message.into(), NoteType::Note));
        self
    }

    /// Get an iterator over every source referenced by this error. May contain
    /// duplicates.
    pub fn sources(&self) -> impl Iterator<Item = SourceId> + '_ {
        std::iter::once(self.at.source).chain(self.labels.iter().map(|(_, span)| span.source))
    }
}

#[expect(unused)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorType {
    Syntax,
    Name,
    Type,
    Evaluation,
}

#[expect(unused)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum NoteType {
    Note,
    Help,
}
