use crate::errors::{Error, ErrorId, ErrorType, Errors, Severity};
use crate::source::Span;

impl Errors {
    pub(crate) fn parse_error(&mut self, at: Span) -> ParseErrors {
        ParseErrors { errors: self, primary: at }
    }
}

pub(crate) struct ParseErrors<'a> {
    errors: &'a mut Errors,
    primary: Span,
}

impl ParseErrors<'_> {
    pub fn expected_case_arm(&mut self) -> ErrorId {
        let error = self
            .error("expected a case arm")
            .with_note("case arms look like `pattern => expression`");
        self.errors.add(error)
    }

    pub fn expected_comma(&mut self) -> ErrorId {
        let error = self.error("expected a comma `,`");
        self.errors.add(error)
    }

    pub fn expected_item(&mut self) -> ErrorId {
        let error = self
            .error("expected an item")
            .with_help("expressions must be inside an item definition: `let name = <expression>`");
        self.errors.add(error)
    }

    pub fn expected_name(&mut self) -> ErrorId {
        let error = self.error("expected a name");
        self.errors.add(error)
    }

    pub fn expected_name_small(&mut self, big: Option<&str>) -> ErrorId {
        let mut error = self.error("expected a value or type name");

        if let Some(big) = big {
            let mut chars = big.chars();
            let fixed: String = chars
                .next()
                .expect("names are at least one character long")
                .to_lowercase()
                .chain(chars)
                .collect();

            error = error
                .with_note("the case of the first letter determines what kind of name it is")
                .with_help(format!("try making the first letter lowercase: `{fixed}`"));
        }

        self.errors.add(error)
    }

    pub fn expected_pattern(&mut self) -> ErrorId {
        let error = self.error("expected a pattern").with_note(
            "patterns include names, wildcards (`_`), and deconstructions (`Variant name1 name2`)",
        );
        self.errors.add(error)
    }

    pub fn item_definition_with_body(&mut self) -> ErrorId {
        let error = self.error("items do not have an expression body").with_note(
            "the expression body is after the `in` keyword, and is only valid in expressions",
        );
        self.errors.add(error)
    }

    pub fn missing_definition(&mut self) -> ErrorId {
        let error = self.error("expected a `=` and a body");
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

    pub fn multiple_record_extensions(&mut self) -> ErrorId {
        let error = self.error("cannot extend multiple records");
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

    pub fn value_definition_without_body(&mut self) -> ErrorId {
        let error = self.error("value definition is missing an expression body")
            .with_help("the definition should be followed with an `in` keyword and an expression: `let name = value in expression`");
        self.errors.add(error)
    }

    fn error(&mut self, title: impl Into<String>) -> Error {
        Error::new(ErrorType::Syntax, Severity::Error, self.primary, title)
    }
}
