use std::collections::BTreeSet;

use bumpalo::Bump;
use log::trace;

use crate::tyck::pretty::Prettifier;
use crate::tyck::types::{Generic, Row};
use crate::tyck::{Scheme, Type};

use super::Solver;

/// Generalization
impl<'a> Solver<'a> {
    pub fn generalize(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Bump,
        explicit: &[Generic],
        ty: &'a Type<'a>,
    ) -> Scheme<'a> {
        trace!("gen {}", pretty.ty(&self.apply(alloc, ty)));

        let mut subst = explicit.iter().copied().collect();
        let ty = alloc.alloc(self.gen_ty(alloc, &mut subst, ty));
        let params = subst.into_iter().collect();
        Scheme { params, ty }
    }

    fn gen_ty(
        &mut self,
        alloc: &'a Bump,
        subst: &mut BTreeSet<Generic>,
        ty: &'a Type<'a>,
    ) -> Type<'a> {
        match ty {
            Type::Invalid(_)
            | Type::Unit
            | Type::Param(_)
            | Type::Boolean
            | Type::Integer
            | Type::Named(_) => ty.clone(),

            Type::Var(v, level) => {
                if let Some(ty) = self.subst.get(v) {
                    self.gen_ty(alloc, subst, ty)
                } else if level.can_generalize(self.level) {
                    let name = Generic::Implicit(*v);
                    subst.insert(name);
                    let ty = alloc.alloc(Type::Param(name));
                    let prev = self.subst.insert(*v, ty);
                    debug_assert!(prev.is_none());
                    ty.clone()
                } else {
                    ty.clone()
                }
            }

            Type::Fun(t, u) => {
                let t = alloc.alloc(self.gen_ty(alloc, subst, t));
                let u = alloc.alloc(self.gen_ty(alloc, subst, u));
                Type::Fun(t, u)
            }

            Type::Record(row) => {
                let row = alloc.alloc(self.gen_row(alloc, subst, row));
                Type::Record(row)
            }

            Type::Variant(row) => {
                let row = alloc.alloc(self.gen_row(alloc, subst, row));
                Type::Variant(row)
            }

            Type::Apply(t, u) => {
                let t = alloc.alloc(self.gen_ty(alloc, subst, t));
                let u = alloc.alloc(self.gen_ty(alloc, subst, u));
                Type::Apply(t, u)
            }
        }
    }

    fn gen_row(
        &mut self,
        alloc: &'a Bump,
        subst: &mut BTreeSet<Generic>,
        row: &'a Row<'a>,
    ) -> Row<'a> {
        match row {
            Row::Invalid(_) | Row::Param(_) | Row::Empty => row.clone(),

            Row::Var(v, level) => {
                if let Some(record) = self.row_subst.get(v) {
                    self.gen_row(alloc, subst, record)
                } else if level.can_generalize(self.level) {
                    let name = Generic::Implicit(*v);
                    subst.insert(name);
                    let row = alloc.alloc(Row::Param(name));
                    let prev = self.row_subst.insert(*v, row);
                    debug_assert!(prev.is_none());
                    row.clone()
                } else {
                    row.clone()
                }
            }

            Row::Extend(label, ty, rest) => {
                let ty = alloc.alloc(self.gen_ty(alloc, subst, ty));
                let rest = alloc.alloc(self.gen_row(alloc, subst, rest));
                Row::Extend(*label, ty, rest)
            }
        }
    }
}
