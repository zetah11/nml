use crate::frontend::names::Name;
use crate::frontend::source::Span;

pub use self::vars::Level;

mod apply;
mod generalize;
mod instantiate;
mod minimize;
mod unify;
mod vars;

use std::collections::{BTreeMap, BTreeSet};

use super::types::{Row, VarKind};
use super::Type;
use super::{Checker, Generic, Scheme};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TypeVar(usize, pub(super) VarKind);

pub struct Solver<'a> {
    subst: BTreeMap<TypeVar, &'a Type<'a>>,
    row_subst: BTreeMap<TypeVar, &'a Row<'a>>,

    counter: usize,
    level: usize,
}

impl<'a> Solver<'a> {
    pub fn new() -> Self {
        Self {
            subst: BTreeMap::new(),
            row_subst: BTreeMap::new(),
            counter: 0,
            level: 0,
        }
    }
}

impl<'a, 'lit> Checker<'a, '_, 'lit, '_> {
    pub fn apply(&self, ty: &'a Type<'a>) -> &'a Type<'a> {
        let ty = self.solver.apply(self.alloc, ty);
        self.alloc.alloc(ty)
    }

    pub fn generalize(&mut self, explicit: &[Generic], ty: &'a Type<'a>) -> Scheme<'a> {
        let mut pretty = self.pretty.build();
        self.solver
            .generalize(&mut pretty, self.alloc, explicit, ty)
    }

    pub fn instantiate_name(&mut self, name: &Name) -> &'a Type<'a> {
        let scheme = self.env.lookup(name);
        let mut pretty = self.pretty.build();
        let ty = self.solver.instantiate(&mut pretty, self.alloc, scheme);
        self.alloc.alloc(ty)
    }

    pub fn minimize(&mut self, keep: &BTreeSet<TypeVar>, ty: &'a Type<'a>) {
        let mut pretty = self.pretty.build();
        self.solver.minimize(&mut pretty, self.alloc, keep, ty)
    }

    pub fn unify(&mut self, at: Span, lhs: &'a Type<'a>, rhs: &'a Type<'a>) {
        let mut pretty = self.pretty.build();
        self.solver
            .unify(&mut pretty, self.alloc, self.errors, at, lhs, rhs)
    }

    pub fn vars_in_ty(&self, ty: &Type) -> BTreeSet<TypeVar> {
        self.solver.vars_in_ty(ty)
    }

    pub fn enter<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.solver.enter();
        let result = f(self);
        self.solver.exit();
        result
    }
}

/// Creating types
impl<'a, 'lit> Checker<'a, '_, 'lit, '_> {
    /// Create a fresh, unique unification variable of type kind
    pub fn fresh(&mut self) -> &'a Type<'a> {
        let ty = self.solver.fresh();
        self.alloc.alloc(ty)
    }

    /// Create a fresh, unique unification variable of row kind
    pub fn fresh_row(&mut self) -> &'a Row<'a> {
        let row = self.solver.fresh_row();
        self.alloc.alloc(row)
    }
}

impl Solver<'_> {
    fn fresh<'l>(&mut self) -> Type<'l> {
        let (var, level) = self.new_var(VarKind::Type);
        Type::Var(var, level)
    }

    fn fresh_row<'l>(&mut self) -> Row<'l> {
        let (var, level) = self.new_var(VarKind::Row);
        Row::Var(var, level)
    }

    fn enter(&mut self) {
        self.level += 1;
    }

    fn exit(&mut self) {
        self.level -= 1;
    }

    fn new_var(&mut self, kind: VarKind) -> (TypeVar, Level) {
        self.counter += 1;
        (TypeVar(self.counter, kind), Level::new(self.level))
    }
}
