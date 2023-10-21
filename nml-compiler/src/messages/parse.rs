use crate::errors::{Error, ErrorId, ErrorType, Errors, Severity};
use crate::source::Span;

impl Errors {
    pub(crate) fn parse_error(&mut self, at: Span) -> ParseErrors {
        ParseErrors {
            errors: self,
            primary: at,
        }
    }
}

pub(crate) struct ParseErrors<'a> {
    errors: &'a mut Errors,
    primary: Span,
}

pub enum NonSmallName<'a> {
    None,
    Universal(&'a str),
}

impl ParseErrors<'_> {
    pub fn ambiguous_infix_operators(&mut self, prev: Span) -> ErrorId {
        let error = self
            .error("ambiguous expression")
            .with_label(prev, "previous infix operator here")
            .with_note("infix operators have no precedence")
            .with_help("disambiguate by adding explicit parentheses");
        self.errors.add(error)
    }

    pub fn constructor_parameters_not_after_name(&mut self) -> ErrorId {
        let error = self
            .error("unexpected tokens")
            .with_help("parameters to a constructor must come outside the constructor name");
        self.errors.add(error)
    }

    pub fn data_parameters_unsupported(&mut self) -> ErrorId {
        let error = self.error("type parameters on data types are not yet supported");
        self.errors.add(error)
    }

    pub fn expected_annotation(&mut self, name: &str) -> ErrorId {
        let error = self
            .error("expected a type annotation")
            .with_help(format!("try using a wildcard type: `{name} : _`"));
        self.errors.add(error)
    }

    pub fn expected_annotated_name(&mut self) -> ErrorId {
        let error = self.error("expected a name with a type annotation");
        self.errors.add(error)
    }

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

    pub fn expected_expr(&mut self) -> ErrorId {
        let error = self.error("expected an expression");
        self.errors.add(error)
    }

    pub fn expected_item(&mut self) -> ErrorId {
        let error = self
            .error("expected an item")
            .with_help("expressions must be inside an item definition: `let name = <expression>`");
        self.errors.add(error)
    }

    pub fn expected_constructor_name(&mut self) -> ErrorId {
        let error = self.error("expected a constructor name");
        self.errors.add(error)
    }

    pub fn expected_name(&mut self) -> ErrorId {
        let error = self.error("expected a name");
        self.errors.add(error)
    }

    pub fn expected_name_small(&mut self, got: NonSmallName) -> ErrorId {
        let mut error = self.error("expected a value or type name");

        match got {
            NonSmallName::None => {}

            NonSmallName::Universal(name) => {
                let fixed = &name[1..]; // the first character is always '\''
                error = error
                    .with_note("value names cannot start with a tick `'`")
                    .with_help(format!("try removing the apostrophe: `{fixed}`"));
            }
        }

        self.errors.add(error)
    }

    pub fn expected_pattern(&mut self) -> ErrorId {
        let error = self.error("expected a pattern").with_note(
            "patterns include names, wildcards (`_`), and deconstructions (`Variant name1 name2`)",
        );
        self.errors.add(error)
    }

    pub fn expected_type(&mut self) -> ErrorId {
        let error = self
            .error("expected a type")
            .with_note("types include wildcards (`_`) and function types (`t -> u`)");
        self.errors.add(error)
    }

    pub fn infix_function(&mut self, name: &str) -> ErrorId {
        let error = self.error(format!("`{name}` is an infix function"));
        self.errors.add(error)
    }

    pub fn item_definition_with_body(&mut self) -> ErrorId {
        let error = self
            .error("items do not have an expression body")
            .with_note(
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

    pub fn multiple_return_type_annotations(&mut self) -> ErrorId {
        let error = self.error("function can only have a single return type annotation");
        self.errors.add(error)
    }

    pub fn postfix_function(&mut self, name: &str) -> ErrorId {
        let error = self.error(format!("`{name}` is a postfix function"));
        self.errors.add(error)
    }

    pub fn record_type_extension(&mut self) -> ErrorId {
        let error = self.error("extensions in record types are not supported");
        self.errors.add(error)
    }

    pub fn record_type_field_definition(&mut self) -> ErrorId {
        let error = self.error("record type field may not be defined");
        self.errors.add(error)
    }

    pub fn scrutinee_in_sum_data_type(&mut self) -> ErrorId {
        let error = self
            .error("data types may not contain a scrutinee")
            .with_help("if you intended this to be a constructor, add an initial bar (`|`)");
        self.errors.add(error)
    }

    pub fn unclosed_brace(&mut self, possible_placement: Span) -> ErrorId {
        let error = self
            .error("unclosed brace")
            .with_label(possible_placement, "expected a `}` here");
        self.errors.add(error)
    }

    pub fn unclosed_paren(&mut self, possible_placement: Span) -> ErrorId {
        let error = self
            .error("unclosed parenthesis")
            .with_label(possible_placement, "expected a `)` here");
        self.errors.add(error)
    }

    pub fn unexpected_function_definition(&mut self) -> ErrorId {
        let error = self.error("unexpected function definition pattern");
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
