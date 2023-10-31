use bumpalo::Bump;
use log::trace;

use crate::errors::Errors;
use crate::names::Label;
use crate::source::Span;
use crate::tyck::pretty::Prettifier;
use crate::tyck::types::Row;
use crate::tyck::{Reporting, Type};

use super::{Level, Solver, TypeVar};

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
            pretty.ty(&self.apply(alloc, lhs)),
            pretty.ty(&self.apply(alloc, rhs))
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

            (Type::Named(n, n_args), Type::Named(m, m_args))
                if n == m && n_args.len() == m_args.len() =>
            {
                for (n_arg, m_arg) in n_args.iter().zip(m_args.iter()) {
                    self.unify_ty(reporting, alloc, n_arg, m_arg);
                }
            }

            (Type::Named(_, args), e @ Type::Invalid(_))
            | (e @ Type::Invalid(_), Type::Named(_, args)) => {
                for arg in args.iter() {
                    self.unify_ty(reporting, alloc, arg, e)
                }
            }

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
                | Type::Named(..)
                | Type::Fun(..)
                | Type::Record(_)
                | Type::Variant(_),
                Type::Unit
                | Type::Boolean
                | Type::Integer
                | Type::Param(_)
                | Type::Named(..)
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
                let tail = row_tail(rest1);
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
            Type::Invalid(_) | Type::Unit | Type::Boolean | Type::Integer | Type::Param(_) => false,

            Type::Named(_, args) => args.iter().any(|ty| self.occurs(var, l1, ty)),

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

/// Rows
impl<'a> Solver<'a> {
    fn rewrite(
        &mut self,
        reporting: &mut Reporting,
        alloc: &'a Bump,
        label: &Label<'a>,
        row: &'a Row<'a>,
        tail: Option<&TypeVar>,
    ) -> (&'a Type<'a>, &'a Row<'a>) {
        match row {
            Row::Empty => {
                let e = reporting
                    .errors
                    .type_error(reporting.at)
                    .no_such_label(reporting.pretty.label(label));

                (alloc.alloc(Type::Invalid(e)), alloc.alloc(Row::Invalid(e)))
            }

            Row::Extend(old, field, rest) if old == label => (*field, *rest),

            Row::Extend(old, field, rest @ Row::Var(alpha, _)) => {
                // Side condition to ensure termination when records with a
                // common tail but distinct prefix are unified
                if tail == Some(alpha) {
                    let id = reporting
                        .errors
                        .type_error(reporting.at)
                        .incompatible_labels(
                            reporting.pretty.label(old),
                            reporting.pretty.label(label),
                        );
                    let e = alloc.alloc(Row::Invalid(id));
                    let et = alloc.alloc(Type::Invalid(id));
                    self.unify_row(reporting, alloc, rest, e);
                    return (et, e);
                }

                let r = self.fresh_record(alloc);
                let t = self.fresh(alloc);
                let rhs = alloc.alloc(Row::Extend(*label, t, r));
                self.unify_row(reporting, alloc, rest, rhs);

                let rest = alloc.alloc(Row::Extend(*old, field, r));
                (t, rest)
            }

            Row::Extend(old, field, rest) => {
                let (label_ty, rest) = self.rewrite(reporting, alloc, label, rest, tail);
                let rest = alloc.alloc(Row::Extend(*old, field, rest));
                (label_ty, rest)
            }

            Row::Invalid(e) => (alloc.alloc(Type::Invalid(*e)), row),

            Row::Var(..) | Row::Param(_) => {
                unreachable!("variables are handled by the unification procedure")
            }
        }
    }
}

pub(super) fn row_tail<'b>(row: &'b Row<'b>) -> Option<&'b TypeVar> {
    match row {
        Row::Var(var, _) => Some(var),
        Row::Extend(_, _, rest) => row_tail(rest),
        Row::Empty | Row::Invalid(_) | Row::Param(_) => None,
    }
}
