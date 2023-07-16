use crate::errors::{Error, ErrorId, ErrorType, Errors, Severity};
use crate::source::Span;

impl Errors {
    pub fn parse_error(&mut self, at: Span) -> ParseErrors {
        ParseErrors { errors: self, primary: at }
    }
}

pub struct ParseErrors<'a> {
    errors: &'a mut Errors,
    primary: Span,
}

impl ParseErrors<'_> {
    pub fn expected_comma(&mut self) -> ErrorId {
        let error = self.error("expected a comma `,`");
        self.errors.add(error)
    }

    pub fn expected_name(&mut self) -> ErrorId {
        let error = self.error("expected a name");
        self.errors.add(error)
    }

    pub fn missing_do(&mut self, kw: &str, possible_placement: Span) -> ErrorId {
        let error = self
            .error(format!("`{kw}` has no corresponding `do`"))
            .with_label(possible_placement, "expected a `do` keyword here");
        self.errors.add(error)
    }

    pub fn missing_end(&mut self, kw: &str, possible_placement: Span) -> ErrorId {
        let error = self
            .error(format!("`{kw}` has no matching `end`"))
            .with_label(possible_placement, "expected an `end` keyword here");
        self.errors.add(error)
    }

    pub fn unclosed_brace(&mut self, possible_placement: Span) -> ErrorId {
        let error =
            self.error("unclosed brace").with_label(possible_placement, "expected a `}` here");
        self.errors.add(error)
    }

    pub fn unclosed_paren(&mut self, possible_placement: Span) -> ErrorId {
        let error = self
            .error("unclosed parenthesis")
            .with_label(possible_placement, "expected a `)` here");
        self.errors.add(error)
    }

    pub fn unexpected_token(&mut self) -> ErrorId {
        let error = self.error("unexpected token");
        self.errors.add(error)
    }

    fn error(&mut self, title: impl Into<String>) -> Error {
        Error::new(ErrorType::Syntax, Severity::Error, self.primary, title)
    }
}
