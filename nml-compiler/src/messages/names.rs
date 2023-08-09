use crate::errors::{Error, ErrorId, ErrorType, Errors, Severity};
use crate::source::Span;

impl Errors {
    pub(crate) fn name_error(&mut self, at: Span) -> NameErrors {
        NameErrors { errors: self, primary: at }
    }
}

pub(crate) struct NameErrors<'a> {
    errors: &'a mut Errors,
    primary: Span,
}

impl NameErrors<'_> {
    pub fn redefined_value(&mut self, prev: Span, name: &str) -> ErrorId {
        let error = self
            .error(format!("redefinition of value `{name}`"))
            .with_label(prev, "previous definition here");
        self.errors.add(error)
    }

    pub fn unapplied_anonymous_variant(&mut self, name: &str) -> ErrorId {
        let error = self
            .error(format!("anonymous variant `{name}` must be applied to a single argument"))
            .with_note("all anonymous variants must take a single argument");
        self.errors.add(error)
    }

    pub fn unknown_name(&mut self, name: &str) -> ErrorId {
        let error = self.error(format!("unknown name `{name}`"));
        self.errors.add(error)
    }

    fn error(&mut self, title: impl Into<String>) -> Error {
        Error::new(ErrorType::Name, Severity::Error, self.primary, title)
    }
}
