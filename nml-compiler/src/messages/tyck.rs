use crate::errors::{Error, ErrorId, ErrorType, Errors, Severity};
use crate::source::Span;

impl Errors {
    pub fn type_error(&mut self, at: Span) -> TypeErrors {
        TypeErrors { errors: self, primary: at }
    }
}

pub struct TypeErrors<'a> {
    errors: &'a mut Errors,
    primary: Span,
}

impl TypeErrors<'_> {
    pub fn inequal_types(&mut self, lhs: String, rhs: String) -> ErrorId {
        let error = self
            .error("incompatible types")
            .with_note(format!("expected `{lhs}`"))
            .with_note(format!(" but got `{rhs}`"));
        self.errors.add(error)
    }

    pub fn incompatible_labels(&mut self, lhs: String, rhs: String) -> ErrorId {
        let error = self
            .error("incompatible record types")
            .with_note(format!("record cannot have both labels `{lhs}` and `{rhs}`"));
        self.errors.add(error)
    }

    pub fn no_such_label(&mut self, label: String) -> ErrorId {
        let error = self.error(format!("record has no field `{label}`"));
        self.errors.add(error)
    }

    pub fn recursive_type(&mut self, var: String, ty: String) -> ErrorId {
        let error = self
            .error("infinite type")
            .with_note(format!("the type `{ty}` contains the type variable `{var}`"))
            .with_note("equating these two would produce an infinite type");
        self.errors.add(error)
    }

    fn error(&mut self, title: impl Into<String>) -> Error {
        Error::new(ErrorType::Type, Severity::Error, self.primary, title)
    }
}
