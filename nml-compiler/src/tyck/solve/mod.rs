use crate::errors::Errors;
use crate::source::Span;

pub use self::vars::Level;

mod rows;
mod vars;

use std::collections::{BTreeMap, BTreeSet};

use bumpalo::Bump;
use log::trace;

use super::pretty::Prettifier;
use super::types::{Generic, Row, VarKind};
use super::{Reporting, Scheme, Type};

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

/// Apply
impl<'a> Solver<'a> {
    /// Apply the current substitution to the given type.
    pub fn apply(&self, alloc: &'a Bump, ty: &'a Type<'a>) -> &'a Type<'a> {
        match ty {
            Type::Invalid(_)
            | Type::Unit
            | Type::Boolean
            | Type::Integer
            | Type::Param(_)
            | Type::Named(_) => ty,

            Type::Var(v, _) => {
                if let Some(ty) = self.subst.get(v) {
                    self.apply(alloc, ty)
                } else {
                    ty
                }
            }

            Type::Fun(t, u) => {
                let t = self.apply(alloc, t);
                let u = self.apply(alloc, u);
                alloc.alloc(Type::Fun(t, u))
            }

            Type::Record(row) => {
                let row = self.apply_row(alloc, row);
                alloc.alloc(Type::Record(row))
            }

            Type::Variant(row) => {
                let row = self.apply_row(alloc, row);
                alloc.alloc(Type::Variant(row))
            }
        }
    }

    fn apply_row(&self, alloc: &'a Bump, row: &'a Row<'a>) -> &'a Row<'a> {
        match row {
            Row::Invalid(_) | Row::Empty | Row::Param(_) => row,

            Row::Var(v, _) => {
                if let Some(record) = self.row_subst.get(v) {
                    self.apply_row(alloc, record)
                } else {
                    row
                }
            }

            Row::Extend(label, ty, rest) => {
                let ty = self.apply(alloc, ty);
                let rest = self.apply_row(alloc, rest);
                alloc.alloc(Row::Extend(*label, ty, rest))
            }
        }
    }
}

/// Instantiation
impl<'a> Solver<'a> {
    pub fn instantiate(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Bump,
        scheme: &Scheme<'a>,
    ) -> &'a Type<'a> {
        trace!(
            "ins {}",
            pretty.scheme(&Scheme {
                params: scheme.params.clone(),
                ty: self.apply(alloc, scheme.ty),
            })
        );

        let subst = scheme
            .params
            .iter()
            .map(|name| (name, self.new_var(VarKind::Type)))
            .collect();

        self.inst_ty(alloc, &subst, scheme.ty)
    }

    fn inst_ty(
        &self,
        alloc: &'a Bump,
        subst: &BTreeMap<&Generic, (TypeVar, Level)>,
        ty: &'a Type<'a>,
    ) -> &'a Type<'a> {
        match ty {
            Type::Invalid(_) | Type::Unit | Type::Boolean | Type::Integer | Type::Named(_) => ty,

            Type::Var(v, _) => {
                if let Some(ty) = self.subst.get(v) {
                    self.inst_ty(alloc, subst, ty)
                } else {
                    ty
                }
            }

            Type::Param(n) => subst
                .get(n)
                .map(|(var, level)| &*alloc.alloc(Type::Var(*var, level.clone())))
                .unwrap_or(ty),

            Type::Fun(t, u) => {
                let t = self.inst_ty(alloc, subst, t);
                let u = self.inst_ty(alloc, subst, u);
                alloc.alloc(Type::Fun(t, u))
            }

            Type::Record(row) => {
                let row = self.inst_row(alloc, subst, row);
                alloc.alloc(Type::Record(row))
            }

            Type::Variant(row) => {
                let row = self.inst_row(alloc, subst, row);
                alloc.alloc(Type::Variant(row))
            }
        }
    }

    fn inst_row(
        &self,
        alloc: &'a Bump,
        subst: &BTreeMap<&Generic, (TypeVar, Level)>,
        row: &'a Row<'a>,
    ) -> &'a Row<'a> {
        match row {
            Row::Invalid(_) | Row::Empty => row,

            Row::Var(v, _) => {
                if let Some(record) = self.row_subst.get(v) {
                    self.inst_row(alloc, subst, record)
                } else {
                    row
                }
            }

            Row::Param(n) => subst
                .get(n)
                .map(|(var, level)| &*alloc.alloc(Row::Var(*var, level.clone())))
                .unwrap_or(row),

            Row::Extend(label, ty, rest) => {
                let ty = self.inst_ty(alloc, subst, ty);
                let rest = self.inst_row(alloc, subst, rest);
                alloc.alloc(Row::Extend(*label, ty, rest))
            }
        }
    }
}

/// Generalization
impl<'a> Solver<'a> {
    pub fn generalize(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Bump,
        ty: &'a Type<'a>,
    ) -> Scheme<'a> {
        trace!("gen {}", pretty.ty(self.apply(alloc, ty)));

        let mut subst = BTreeSet::new();
        let ty = self.gen_ty(alloc, &mut subst, ty);
        let params = subst.into_iter().collect();
        Scheme { params, ty }
    }

    fn gen_ty(
        &mut self,
        alloc: &'a Bump,
        subst: &mut BTreeSet<Generic>,
        ty: &'a Type<'a>,
    ) -> &'a Type<'a> {
        match ty {
            Type::Invalid(_)
            | Type::Unit
            | Type::Param(_)
            | Type::Boolean
            | Type::Integer
            | Type::Named(_) => ty,

            Type::Var(v, level) => {
                if let Some(ty) = self.subst.get(v) {
                    self.gen_ty(alloc, subst, ty)
                } else if level.can_generalize(self.level) {
                    let name = Generic(*v);
                    subst.insert(name);
                    let ty = alloc.alloc(Type::Param(name));
                    let prev = self.subst.insert(*v, ty);
                    debug_assert!(prev.is_none());
                    ty
                } else {
                    ty
                }
            }

            Type::Fun(t, u) => {
                let t = self.gen_ty(alloc, subst, t);
                let u = self.gen_ty(alloc, subst, u);
                alloc.alloc(Type::Fun(t, u))
            }

            Type::Record(row) => {
                let row = self.gen_row(alloc, subst, row);
                alloc.alloc(Type::Record(row))
            }

            Type::Variant(row) => {
                let row = self.gen_row(alloc, subst, row);
                alloc.alloc(Type::Variant(row))
            }
        }
    }

    fn gen_row(
        &mut self,
        alloc: &'a Bump,
        subst: &mut BTreeSet<Generic>,
        row: &'a Row<'a>,
    ) -> &'a Row<'a> {
        match row {
            Row::Invalid(_) | Row::Param(_) | Row::Empty => row,

            Row::Var(v, level) => {
                if let Some(record) = self.row_subst.get(v) {
                    self.gen_row(alloc, subst, record)
                } else if level.can_generalize(self.level) {
                    let name = Generic(*v);
                    subst.insert(name);
                    let row = alloc.alloc(Row::Param(name));
                    let prev = self.row_subst.insert(*v, row);
                    debug_assert!(prev.is_none());
                    row
                } else {
                    row
                }
            }

            Row::Extend(label, ty, rest) => {
                let ty = self.gen_ty(alloc, subst, ty);
                let rest = self.gen_row(alloc, subst, rest);
                alloc.alloc(Row::Extend(*label, ty, rest))
            }
        }
    }
}

/// Minimization
impl<'a> Solver<'a> {
    /// Unify all of the unbound row variables with the empty row in the given
    /// type, fixing/minimizing it to its current labels.
    pub fn minimize(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Bump,
        keep: &BTreeSet<TypeVar>,
        ty: &'a Type<'a>,
    ) {
        trace!(
            "min {} -- keep [{}]",
            pretty.ty(self.apply(alloc, ty)),
            keep.iter()
                .map(|v| pretty.var(v, None))
                .collect::<Vec<_>>()
                .join(", ")
        );

        self.minimize_ty(alloc, keep, ty);
    }

    fn minimize_ty(&mut self, alloc: &'a Bump, keep: &BTreeSet<TypeVar>, ty: &'a Type<'a>) {
        match ty {
            Type::Invalid(_)
            | Type::Unit
            | Type::Boolean
            | Type::Integer
            | Type::Param(_)
            | Type::Named(_) => {}

            Type::Var(v, _) => {
                if keep.contains(v) {
                } else if let Some(ty) = self.subst.get(v) {
                    self.minimize_ty(alloc, keep, ty);
                }
            }

            Type::Fun(t, u) => {
                self.minimize_ty(alloc, keep, t);
                self.minimize_ty(alloc, keep, u);
            }

            Type::Record(row) | Type::Variant(row) => self.minimize_row(alloc, keep, row),
        }
    }

    fn minimize_row(&mut self, alloc: &'a Bump, keep: &BTreeSet<TypeVar>, row: &'a Row<'a>) {
        match row {
            Row::Invalid(_) | Row::Empty | Row::Param(_) => {}

            Row::Var(v, _) => {
                if keep.contains(v) {
                } else if let Some(row) = self.row_subst.get(v) {
                    self.minimize_row(alloc, keep, row)
                } else {
                    let row = alloc.alloc(Row::Empty);
                    let prev = self.row_subst.insert(*v, row);
                    debug_assert!(prev.is_none());
                }
            }

            Row::Extend(_, ty, rest) => {
                self.minimize_ty(alloc, keep, ty);
                self.minimize_row(alloc, keep, rest);
            }
        }
    }
}

/// Unification
impl<'a> Solver<'a> {
    pub fn unify(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Bump,
        errors: &mut Errors,
        at: Span,
        lhs: &'a Type<'a>,
        rhs: &'a Type<'a>,
    ) {
        trace!(
            "uni {}  ~  {}",
            pretty.ty(self.apply(alloc, lhs)),
            pretty.ty(self.apply(alloc, rhs))
        );
        self.unify_ty(&mut Reporting { pretty, errors, at }, alloc, lhs, rhs)
    }

    fn unify_ty(
        &mut self,
        reporting: &mut Reporting,
        alloc: &'a Bump,
        lhs: &'a Type<'a>,
        rhs: &'a Type<'a>,
    ) {
        match (lhs, rhs) {
            (Type::Unit, Type::Unit) => {}
            (Type::Unit, Type::Invalid(_)) | (Type::Invalid(_), Type::Unit) => {}

            (Type::Boolean, Type::Boolean) => {}
            (Type::Boolean, Type::Invalid(_)) | (Type::Invalid(_), Type::Boolean) => {}

            (Type::Integer, Type::Integer) => {}
            (Type::Integer, Type::Invalid(_)) | (Type::Invalid(_), Type::Integer) => {}

            (Type::Param(t), Type::Param(u)) if t == u => {}
            (Type::Param(_), Type::Invalid(_)) | (Type::Invalid(_), Type::Param(_)) => {}

            (Type::Named(n), Type::Named(m)) if n == m => {}
            (Type::Named(_), Type::Invalid(_)) | (Type::Invalid(_), Type::Named(_)) => {}

            (Type::Fun(t1, u1), Type::Fun(t2, u2)) => {
                self.unify_ty(reporting, alloc, t1, t2);
                self.unify_ty(reporting, alloc, u1, u2);
            }
            (Type::Fun(t, u), e @ Type::Invalid(_)) | (e @ Type::Invalid(_), Type::Fun(t, u)) => {
                self.unify_ty(reporting, alloc, t, e);
                self.unify_ty(reporting, alloc, u, e);
            }

            (Type::Record(row1), Type::Record(row2))
            | (Type::Variant(row1), Type::Variant(row2)) => {
                self.unify_row(reporting, alloc, row1, row2)
            }

            (Type::Record(row), Type::Invalid(e))
            | (Type::Invalid(e), Type::Record(row))
            | (Type::Variant(row), Type::Invalid(e))
            | (Type::Invalid(e), Type::Variant(row)) => {
                let e = alloc.alloc(Row::Invalid(*e));
                self.unify_row(reporting, alloc, row, e)
            }

            (Type::Var(var, level), ty) | (ty, Type::Var(var, level)) => {
                if let Some(rhs) = self.subst.get(var) {
                    self.unify_ty(reporting, alloc, ty, rhs)
                } else {
                    self.set(reporting, alloc, var, level, ty)
                }
            }

            (Type::Invalid(_), Type::Invalid(_)) => {}

            // Use the exhaustiveness check to ensure termination when unifying
            // with error types
            (
                Type::Unit
                | Type::Boolean
                | Type::Integer
                | Type::Param(_)
                | Type::Named(_)
                | Type::Fun(..)
                | Type::Record(_)
                | Type::Variant(_),
                Type::Unit
                | Type::Boolean
                | Type::Integer
                | Type::Param(_)
                | Type::Named(_)
                | Type::Fun(..)
                | Type::Record(_)
                | Type::Variant(_),
            ) => {
                let e = {
                    let lhs = reporting.pretty.ty(lhs);
                    let rhs = reporting.pretty.ty(rhs);
                    let e = reporting
                        .errors
                        .type_error(reporting.at)
                        .inequal_types(lhs, rhs);
                    alloc.alloc(Type::Invalid(e))
                };

                self.unify_ty(reporting, alloc, lhs, e);
                self.unify_ty(reporting, alloc, e, rhs);
            }
        }
    }

    fn unify_row(
        &mut self,
        reporting: &mut Reporting,
        alloc: &'a Bump,
        lhs: &'a Row<'a>,
        rhs: &'a Row<'a>,
    ) {
        match (lhs, rhs) {
            (Row::Empty, Row::Empty) => {}
            (Row::Empty, Row::Invalid(_)) | (Row::Invalid(_), Row::Empty) => {}

            (Row::Extend(label, ty1, rest1), row2 @ Row::Extend(..)) => {
                let tail = Self::row_tail(rest1);
                let (ty2, rest2) = self.rewrite(reporting, alloc, label, row2, tail);
                self.unify_ty(reporting, alloc, ty1, ty2);
                self.unify_row(reporting, alloc, rest1, rest2);
            }

            (Row::Extend(_, ty, rest), e @ Row::Invalid(id))
            | (e @ Row::Invalid(id), Row::Extend(_, ty, rest)) => {
                let et = alloc.alloc(Type::Invalid(*id));
                self.unify_ty(reporting, alloc, ty, et);
                self.unify_row(reporting, alloc, rest, e);
            }

            (Row::Param(n), Row::Param(m)) if n == m => {}
            (Row::Param(_), Row::Invalid(_)) | (Row::Invalid(_), Row::Param(_)) => {}

            (Row::Var(var, level), record) | (record, Row::Var(var, level)) => {
                if let Some(rhs) = self.row_subst.get(var) {
                    self.unify_row(reporting, alloc, record, rhs)
                } else {
                    self.set_record(reporting, alloc, var, level, record)
                }
            }

            (Row::Invalid(_), Row::Invalid(_)) => {}

            (
                Row::Empty | Row::Extend(..) | Row::Param(_),
                Row::Empty | Row::Extend(..) | Row::Param(_),
            ) => {
                let e = {
                    let lhs = reporting.pretty.record(lhs);
                    let rhs = reporting.pretty.record(rhs);
                    let e = reporting
                        .errors
                        .type_error(reporting.at)
                        .inequal_types(lhs, rhs);
                    alloc.alloc(Row::Invalid(e))
                };

                self.unify_row(reporting, alloc, lhs, e);
                self.unify_row(reporting, alloc, e, rhs);
            }
        }
    }

    fn set(
        &mut self,
        reporting: &mut Reporting,
        alloc: &'a Bump,
        var: &TypeVar,
        level: &Level,
        ty: &'a Type<'a>,
    ) {
        if let Type::Var(v, l2) = ty {
            l2.set_min(level);
            if v == var {
                return;
            }
        }

        // Occurs check
        if self.occurs(var, level, ty) {
            let ty = {
                let var = reporting.pretty.var(var, Some(level));
                let ty = reporting.pretty.ty(ty);
                let e = reporting
                    .errors
                    .type_error(reporting.at)
                    .recursive_type(var, ty);
                alloc.alloc(Type::Invalid(e))
            };

            return self.set(reporting, alloc, var, level, ty);
        }

        let prev = self.subst.insert(*var, ty);
        debug_assert!(prev.is_none(), "overwrote previous unification variable");
    }

    fn set_record(
        &mut self,
        reporting: &mut Reporting,
        alloc: &'a Bump,
        var: &TypeVar,
        level: &Level,
        record: &'a Row<'a>,
    ) {
        if let Row::Var(v, l2) = record {
            l2.set_min(level);
            if v == var {
                return;
            }
        }

        if self.occurs_row(var, level, record) {
            let record = {
                let var = reporting.pretty.var(var, Some(level));
                let ty = reporting.pretty.record(record);
                let e = reporting
                    .errors
                    .type_error(reporting.at)
                    .recursive_type(var, ty);
                alloc.alloc(Row::Invalid(e))
            };

            return self.set_record(reporting, alloc, var, level, record);
        }

        let prev = self.row_subst.insert(*var, record);
        debug_assert!(
            prev.is_none(),
            "overwrote previous record row unificiation variable"
        );
    }

    fn occurs(&self, var: &TypeVar, l1: &Level, ty: &Type) -> bool {
        match ty {
            Type::Invalid(_)
            | Type::Unit
            | Type::Boolean
            | Type::Integer
            | Type::Param(_)
            | Type::Named(_) => false,

            Type::Var(war, l2) => {
                if let Some(ty) = self.subst.get(war) {
                    self.occurs(var, l1, ty)
                } else {
                    l2.set_min(l1);
                    var == war
                }
            }

            Type::Record(row) => self.occurs_row(var, l1, row),
            Type::Variant(row) => self.occurs_row(var, l1, row),

            Type::Fun(t, u) => self.occurs(var, l1, t) || self.occurs(var, l1, u),
        }
    }

    fn occurs_row(&self, var: &TypeVar, l1: &Level, row: &Row) -> bool {
        match row {
            Row::Invalid(_) | Row::Empty | Row::Param(_) => false,

            Row::Var(war, l2) => {
                if let Some(record) = self.row_subst.get(war) {
                    self.occurs_row(var, l1, record)
                } else {
                    l2.set_min(l1);
                    var == war
                }
            }

            Row::Extend(_, ty, rest) => self.occurs(var, l1, ty) || self.occurs_row(var, l1, rest),
        }
    }
}
