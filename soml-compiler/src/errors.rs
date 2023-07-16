use std::collections::HashMap;

use crate::source::Span;

/// Identifies a particular reported message.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ErrorId(usize);

impl ErrorId {
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

/// Stores reported errors.
#[derive(Debug, Default)]
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

    /// Drain this error store of its errors.
    pub fn drain(&mut self) -> impl Iterator<Item = (ErrorId, Error)> + '_ {
        self.num_errors = 0;
        self.num_warnings = 0;
        self.num_infos = 0;
        self.errors.drain()
    }
}

#[derive(Debug)]
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
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ErrorType {
    Syntax,
    Name,
    Type,
    Evaluation,
}

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
