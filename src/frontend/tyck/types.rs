use std::collections::BTreeMap;

use super::solve::{Level, TypeVar};
use crate::frontend::errors::ErrorId;
use crate::frontend::names::{Label, Name};

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum Type<'a> {
    Invalid(ErrorId),

    Var(TypeVar, Level),
    Param(Generic),

    Named(Name),

    Unit,
    Integer,
    Arrow,
    Record(&'a Row<'a>),

    #[allow(unused, reason = "eventually, the anonymous sums will rise!")]
    Variant(&'a Row<'a>),

    Apply(&'a Type<'a>, &'a Type<'a>),
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum Row<'a> {
    Invalid(ErrorId),
    Empty,
    Var(TypeVar, Level),
    Param(Generic),
    Extend(Label<'a>, &'a Type<'a>, &'a Row<'a>),
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum VarKind {
    Type,
    Row,
}

#[derive(Clone, Copy, Debug, Eq, Ord, Hash, PartialEq, PartialOrd)]
pub enum Generic {
    Implicit(TypeVar),
    Ticked(Name),
}

#[derive(Clone, Debug)]
pub struct Scheme<'a> {
    pub params: Vec<Generic>,
    pub ty: &'a Type<'a>,
}

impl<'a> Scheme<'a> {
    pub fn mono(ty: &'a Type<'a>) -> Self {
        Self {
            params: Vec::new(),
            ty,
        }
    }

    pub fn is_mono(&self) -> bool {
        self.params.is_empty()
    }

    /// Use the type parameters from this scheme on another type.
    pub fn onto(&self, ty: &'a Type<'a>) -> Self {
        Self {
            params: self.params.clone(),
            ty,
        }
    }
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

    pub fn overwrite(&mut self, name: Name, scheme: Scheme<'a>) {
        let prev = self.context.insert(name, scheme);
        debug_assert!(prev.is_some());
    }

    pub fn lookup(&self, name: &Name) -> &Scheme<'a> {
        self.context
            .get(name)
            .expect("all names are defined before use")
    }

    pub(super) fn try_lookup(&self, name: &Name) -> Option<&Scheme<'a>> {
        self.context.get(name)
    }
}
