use std::collections::BTreeMap;
use std::fmt;

use malachite::Integer;

use super::solve::{Level, TypeVar};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Name(String);

impl Name {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Label(String);

impl Label {
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ErrorId(String);

impl ErrorId {
    pub fn new(error: impl Into<String>) -> Self {
        Self(error.into())
    }
}

impl fmt::Display for ErrorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug)]
pub enum Expr<'a> {
    /// Something fishy
    Invalid(ErrorId),

    /// Variable name
    Var(Name),

    /// `true` or `false`
    Bool(bool),

    /// Some integer
    Number(Integer),

    /// `if x then y else z`
    If(&'a Expr<'a>, &'a Expr<'a>, &'a Expr<'a>),

    /// `x.a`
    Field(&'a Expr<'a>, Label),

    /// `{}`
    Empty,

    /// `x with { a = y }`
    Extend(&'a Expr<'a>, Label, &'a Expr<'a>),

    /// `x \ a`
    Restrict(&'a Expr<'a>, Label),

    /// `x y`
    Apply(&'a Expr<'a>, &'a Expr<'a>),

    /// `a => x`
    Lambda(Name, &'a Expr<'a>),

    /// `let a = x in y`
    Let(Name, &'a Expr<'a>, &'a Expr<'a>),
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum Type<'a> {
    Invalid(ErrorId),

    Var(TypeVar, Level),
    Param(Name),

    Boolean,
    Integer,
    Fun(&'a Type<'a>, &'a Type<'a>),

    Record(&'a Type<'a>),
    Empty,
    Extend(Label, &'a Type<'a>, &'a Type<'a>),
}

#[derive(Debug)]
pub struct Scheme<'a> {
    pub params: Vec<Name>,
    pub ty: &'a Type<'a>,
}

#[derive(Debug, Default)]
pub struct Env<'a> {
    context: BTreeMap<Name, Scheme<'a>>,
}

impl<'a> Env<'a> {
    pub fn new() -> Self {
        Self {
            context: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, name: Name, scheme: Scheme<'a>) {
        let prev = self.context.insert(name, scheme);
        debug_assert!(prev.is_none());
    }

    pub fn lookup(&self, name: &Name) -> &Scheme<'a> {
        self.context
            .get(name)
            .expect("all names are defined before use")
    }
}
