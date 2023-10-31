pub use self::vars::Level;

mod apply;
mod generalize;
mod instantiate;
mod minimize;
mod unify;
mod vars;

use std::collections::BTreeMap;

use bumpalo::Bump;

use super::types::{Row, VarKind};
use super::Type;

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

    pub fn fresh(&mut self, alloc: &'a Bump) -> &'a Type<'a> {
        let (var, level) = self.new_var(VarKind::Type);
        alloc.alloc(Type::Var(var, level))
    }

    pub fn fresh_record(&mut self, alloc: &'a Bump) -> &'a Row<'a> {
        let (var, level) = self.new_var(VarKind::Row);
        alloc.alloc(Row::Var(var, level))
    }

    pub fn enter(&mut self) {
        self.level += 1;
    }

    pub fn exit(&mut self) {
        self.level -= 1;
    }

    fn new_var(&mut self, kind: VarKind) -> (TypeVar, Level) {
        self.counter += 1;
        (TypeVar(self.counter, kind), Level::new(self.level))
    }
}
