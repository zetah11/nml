use std::collections::BTreeSet;

use bumpalo::Bump;
use log::trace;

use crate::frontend::tyck::pretty::Prettifier;
use crate::frontend::tyck::types::Row;
use crate::frontend::tyck::Type;

use super::{Solver, TypeVar};

/// Minimization
impl<'a> Solver<'a> {
    /// Unify all of the unbound row variables with the empty row in the given
    /// type, fixing/minimizing it to its current labels.
    pub(super) fn minimize(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Bump,
        keep: &BTreeSet<TypeVar>,
        ty: &'a Type<'a>,
    ) {
        trace!(
            "min {} -- keep [{}]",
            pretty.ty(&self.apply(alloc, ty)),
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
            | Type::Integer
            | Type::Param(_)
            | Type::Named(_)
            | Type::Arrow => {}

            Type::Var(v, _) => {
                if keep.contains(v) {
                } else if let Some(ty) = self.subst.get(v) {
                    self.minimize_ty(alloc, keep, ty);
                }
            }

            Type::Apply(t, u) => {
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
