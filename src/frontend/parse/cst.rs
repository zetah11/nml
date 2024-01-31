use crate::frontend::errors::ErrorId;
use crate::frontend::source::Span;

#[derive(Clone, Debug)]
pub struct Thing<'a, 'src> {
    pub node: Node<'a, 'src>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Node<'a, 'src> {
    Invalid(ErrorId),

    Wildcard,
    Ellipses,
    Infix,
    Postfix,

    Name(Name<'src>),
    Number(&'src str),

    Group(&'a Thing<'a, 'src>),

    Let {
        kw: (LetKw, Span),
        defs: (ValueDef<'a, 'src>, Vec<ValueDef<'a, 'src>>),

        /// The expression after the `in`
        within: Option<&'a Thing<'a, 'src>>,
    },

    Case(Option<&'a Thing<'a, 'src>>, &'a Thing<'a, 'src>),

    Alt(Vec<&'a Thing<'a, 'src>>),
    And(&'a Thing<'a, 'src>, &'a Thing<'a, 'src>),
    Arrow(&'a Thing<'a, 'src>, &'a Thing<'a, 'src>),

    Apply(Vec<&'a Thing<'a, 'src>>),
    Field(&'a Thing<'a, 'src>, Vec<(Name<'src>, Span)>),

    Anno(&'a Thing<'a, 'src>, &'a Thing<'a, 'src>),

    Record {
        defs: Vec<ValueDef<'a, 'src>>,
    },
}

#[derive(Clone, Debug)]
pub enum LetKw {
    Data,
    Let,
}

#[derive(Clone, Debug)]
pub enum Name<'src> {
    Normal(&'src str),
    Universal(&'src str),
}

/// A single definition (i.e. a `pattern = expression` sequence).
#[derive(Clone, Debug)]
pub struct ValueDef<'a, 'src> {
    /// The span of the whole definition.
    pub span: Span,

    /// The pattern part (the thing before the `=`)
    pub pattern: &'a Thing<'a, 'src>,

    /// The definition part (the thing after the `=`) or `None` if there was no
    /// `=`.
    pub definition: Option<&'a Thing<'a, 'src>>,
}
