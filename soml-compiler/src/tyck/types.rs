use std::collections::BTreeMap;

use super::solve::{Level, TypeVar};
use crate::errors::ErrorId;
use crate::names::{Label, Name};

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum Type<'a> {
    Invalid(ErrorId),

    Var(TypeVar, Level),
    Param(Generic),

    Named(Name),

    Boolean,
    Integer,
    Fun(&'a Type<'a>, &'a Type<'a>),

    Record(&'a Row<'a>),
    Variant(&'a Row<'a>),
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum Row<'a> {
    Invalid(ErrorId),
    Empty,
    Var(TypeVar, Level),
    Param(Generic),
    Extend(Label, &'a Type<'a>, &'a Row<'a>),
}

#[derive(Clone, Copy, Debug, Eq, Ord, Hash, PartialEq, PartialOrd)]
pub struct Generic(pub TypeVar);

#[derive(Debug)]
pub struct Scheme<'a> {
    pub params: Vec<Generic>,
    pub ty: &'a Type<'a>,
}

impl<'a> Scheme<'a> {
    pub fn mono(ty: &'a Type<'a>) -> Self {
        Self { params: Vec::new(), ty }
    }
}

#[derive(Debug, Default)]
pub struct Env<'a> {
    context: BTreeMap<Name, Scheme<'a>>,
}

impl<'a> Env<'a> {
    pub fn new() -> Self {
        Self { context: BTreeMap::new() }
    }

    pub fn insert(&mut self, name: Name, scheme: Scheme<'a>) {
        let prev = self.context.insert(name, scheme);
        debug_assert!(prev.is_none());
    }

    pub fn lookup(&self, name: &Name) -> &Scheme<'a> {
        self.context.get(name).expect("all names are defined before use")
    }
}
