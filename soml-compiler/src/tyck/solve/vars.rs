use std::cell::Cell;
use std::collections::BTreeSet;
use std::rc::Rc;

use super::{Solver, TypeVar};
use crate::tyck::{Row, Type};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Level(Rc<Cell<usize>>);

impl Level {
    pub fn new(level: usize) -> Self {
        Self(Rc::new(Cell::new(level)))
    }

    pub fn can_generalize(&self, current: usize) -> bool {
        self.0.get() > current
    }

    /// Set `self` to be the minimum level of `self` and `other`
    pub fn set_min(&self, other: &Self) {
        self.0.set(self.0.get().min(other.0.get()))
    }

    pub fn as_usize(&self) -> usize {
        self.0.get()
    }
}

impl Solver<'_> {
    /// Return a set of all (currently) unbound type variables referenced by a
    /// particular type.
    pub fn vars_in_ty(&self, ty: &Type) -> BTreeSet<TypeVar> {
        match ty {
            Type::Invalid(_)
            | Type::Unit
            | Type::Boolean
            | Type::Integer
            | Type::Param(_)
            | Type::Named(_) => BTreeSet::new(),

            Type::Var(var, _) => {
                if let Some(ty) = self.subst.get(var) {
                    self.vars_in_ty(ty)
                } else {
                    BTreeSet::from([*var])
                }
            }

            Type::Fun(t, u) => self.vars_in_ty(t).union(&self.vars_in_ty(u)).copied().collect(),
            Type::Record(row) | Type::Variant(row) => self.vars_in_row(row),
        }
    }

    fn vars_in_row(&self, row: &Row) -> BTreeSet<TypeVar> {
        match row {
            Row::Invalid(_) | Row::Empty | Row::Param(_) => BTreeSet::new(),

            Row::Var(var, _) => {
                if let Some(row) = self.row_subst.get(var) {
                    self.vars_in_row(row)
                } else {
                    BTreeSet::from([*var])
                }
            }

            Row::Extend(_, ty, rest) => {
                self.vars_in_ty(ty).union(&self.vars_in_row(rest)).copied().collect()
            }
        }
    }
}
