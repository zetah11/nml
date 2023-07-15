use std::collections::BTreeMap;

use malachite::Integer;

use crate::errors::ErrorId;
use crate::names::{Label, Name};
use crate::source::Span;

use super::solve::{Level, TypeVar};

#[derive(Clone, Debug)]
pub struct Expr<'a> {
    pub node: ExprNode<'a>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprNode<'a> {
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
    Param(Generic),

    Boolean,
    Integer,
    Fun(&'a Type<'a>, &'a Type<'a>),

    Record(&'a RecordRow<'a>),
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum RecordRow<'a> {
    Invalid(ErrorId),
    Empty,
    Var(TypeVar, Level),
    Param(Generic),
    Extend(Label, &'a Type<'a>, &'a RecordRow<'a>),
}

#[derive(Clone, Copy, Debug, Eq, Ord, Hash, PartialEq, PartialOrd)]
pub struct Generic(pub TypeVar);

#[derive(Debug)]
pub struct Scheme<'a> {
    pub params: Vec<Generic>,
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
