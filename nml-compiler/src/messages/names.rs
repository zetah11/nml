use crate::errors::{Error, ErrorId, ErrorType, Errors, Severity};
use crate::source::Span;

impl Errors {
    pub(crate) fn name_error(&mut self, at: Span) -> NameErrors {
        NameErrors {
            errors: self,
            primary: at,
        }
    }
}

pub(crate) struct NameErrors<'a> {
    errors: &'a mut Errors,
    primary: Span,
}

impl NameErrors<'_> {
    pub fn affii_disagree(&mut self, prev: Span) -> ErrorId {
        let error = self
            .error("name bindings have different affixes")
            .with_label(prev, "previous binding here");
        self.errors.add(error)
    }

    pub fn implicit_type_var_in_data(&mut self) -> ErrorId {
        let error = self
            .error("implicit type variables are not allowed in data types")
            .with_help("explicitly add a type parameter with a normal type name to the data type");
        self.errors.add(error)
    }

    pub fn or_patterns_disagree<'s>(&mut self, names: impl Iterator<Item = &'s str>) -> ErrorId {
        let names: Vec<_> = names.map(|name| format!("`{name}`")).collect();
        let s = if names.len() == 1 { "" } else { "s" };
        let names = names.join(", ");

        let error = self.error(format!(
            "name{s} {names} must be bound in both sides of the or-pattern"
        ));
        self.errors.add(error)
    }

    pub fn redefined_type(&mut self, prev: Span, name: &str) -> ErrorId {
        let error = self
            .error(format!("redefinition of type `{name}`"))
            .with_label(prev, "previous definition here");
        self.errors.add(error)
    }

    pub fn redefined_value(&mut self, prev: Span, name: &str) -> ErrorId {
        let error = self
            .error(format!("redefinition of value `{name}`"))
            .with_label(prev, "previous definition here");
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
