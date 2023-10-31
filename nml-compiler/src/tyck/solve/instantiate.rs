use std::collections::BTreeMap;

use bumpalo::Bump;
use log::trace;

use crate::tyck::pretty::Prettifier;
use crate::tyck::types::{Generic, Row, VarKind};
use crate::tyck::{Scheme, Type};

use super::{Level, Solver, TypeVar};

/// Instantiation
impl<'a> Solver<'a> {
    pub fn instantiate(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Bump,
        scheme: &Scheme<'a>,
    ) -> Type<'a> {
        trace!(
            "ins {}",
            pretty.scheme(&Scheme {
                params: scheme.params.clone(),
                ty: alloc.alloc(self.apply(alloc, scheme.ty)),
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
    ) -> Type<'a> {
        match ty {
            Type::Invalid(_) | Type::Unit | Type::Boolean | Type::Integer | Type::Named(_) => {
                ty.clone()
            }

            Type::Var(v, _) => {
                if let Some(ty) = self.subst.get(v) {
                    self.inst_ty(alloc, subst, ty)
                } else {
                    ty.clone()
                }
            }

            Type::Param(n) => subst
                .get(n)
                .map(|(var, level)| &*alloc.alloc(Type::Var(*var, level.clone())))
                .unwrap_or(ty)
                .clone(),

            Type::Fun(t, u) => {
                let t = alloc.alloc(self.inst_ty(alloc, subst, t));
                let u = alloc.alloc(self.inst_ty(alloc, subst, u));
                Type::Fun(t, u)
            }

            Type::Record(row) => {
                let row = alloc.alloc(self.inst_row(alloc, subst, row));
                Type::Record(row)
            }

            Type::Variant(row) => {
                let row = alloc.alloc(self.inst_row(alloc, subst, row));
                Type::Variant(row)
            }

            Type::Apply(t, u) => {
                let t = alloc.alloc(self.inst_ty(alloc, subst, t));
                let u = alloc.alloc(self.inst_ty(alloc, subst, u));
                Type::Apply(t, u)
            }
        }
    }

    fn inst_row(
        &self,
        alloc: &'a Bump,
        subst: &BTreeMap<&Generic, (TypeVar, Level)>,
        row: &'a Row<'a>,
    ) -> Row<'a> {
        match row {
            Row::Invalid(_) | Row::Empty => row.clone(),

            Row::Var(v, _) => {
                if let Some(record) = self.row_subst.get(v) {
                    self.inst_row(alloc, subst, record)
                } else {
                    row.clone()
                }
            }

            Row::Param(n) => subst
                .get(n)
                .map(|(var, level)| &*alloc.alloc(Row::Var(*var, level.clone())))
                .unwrap_or(row)
                .clone(),

            Row::Extend(label, ty, rest) => {
                let ty = alloc.alloc(self.inst_ty(alloc, subst, ty));
                let rest = alloc.alloc(self.inst_row(alloc, subst, rest));
                Row::Extend(*label, ty, rest)
            }
        }
    }
}
