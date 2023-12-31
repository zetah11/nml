use crate::errors::ErrorId;
use crate::source::Span;

#[derive(Clone, Debug)]
pub struct Thing<'a> {
    pub node: Node<'a>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum Node<'a> {
    Invalid(ErrorId),

    Wildcard,
    Ellipses,
    Infix,
    Postfix,

    Name(Name<'a>),
    Number(&'a str),

    Group(&'a Thing<'a>),

    Let {
        kw: (LetKw, Span),
        defs: (ValueDef<'a>, Vec<ValueDef<'a>>),

        /// The expression after the `in`
        within: Option<&'a Thing<'a>>,
    },

    Case(Option<&'a Thing<'a>>, &'a Thing<'a>),

    Alt(Vec<&'a Thing<'a>>),
    Arrow(&'a Thing<'a>, &'a Thing<'a>),

    Apply(Vec<&'a Thing<'a>>),
    Field(&'a Thing<'a>, Vec<(Name<'a>, Span)>),

    Anno(&'a Thing<'a>, &'a Thing<'a>),

    Record {
        defs: Vec<ValueDef<'a>>,
    },
}

#[derive(Clone, Debug)]
pub enum LetKw {
    Data,
    Let,
}

#[derive(Clone, Debug)]
pub enum Name<'a> {
    Normal(&'a str),
    Universal(&'a str),
}

/// A single definition (i.e. a `pattern = expression` sequence).
#[derive(Clone, Debug)]
pub struct ValueDef<'a> {
    /// The span of the whole definition.
    pub span: Span,

    /// The pattern part (the thing before the `=`)
    pub pattern: &'a Thing<'a>,

    /// The definition part (the thing after the `=`) or `None` if there was no
    /// `=`.
    pub definition: Option<&'a Thing<'a>>,
}
