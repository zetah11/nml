use bumpalo::Bump;

use crate::frontend::tyck::types::Row;
use crate::frontend::tyck::Type;

use super::Solver;

/// Apply
impl<'a> Solver<'a> {
    /// Apply the current substitution to the given type, producing a new type
    /// where the only remaining unification variables are the ones not yet
    /// solved.
    pub(super) fn apply(&self, alloc: &'a Bump, ty: &'a Type<'a>) -> Type<'a> {
        match ty {
            Type::Invalid(_)
            | Type::Unit
            | Type::Integer
            | Type::Param(_)
            | Type::Named(_)
            | Type::Arrow => ty.clone(),

            Type::Var(v, _) => {
                if let Some(ty) = self.subst.get(v) {
                    self.apply(alloc, ty)
                } else {
                    ty.clone()
                }
            }

            Type::Record(row) => {
                let row = alloc.alloc(self.apply_row(alloc, row));
                Type::Record(row)
            }

            Type::Variant(row) => {
                let row = alloc.alloc(self.apply_row(alloc, row));
                Type::Variant(row)
            }

            Type::Apply(t, u) => {
                let t = alloc.alloc(self.apply(alloc, t));
                let u = alloc.alloc(self.apply(alloc, u));
                Type::Apply(t, u)
            }
        }
    }

    fn apply_row(&self, alloc: &'a Bump, row: &'a Row<'a>) -> Row<'a> {
        match row {
            Row::Invalid(_) | Row::Empty | Row::Param(_) => row.clone(),

            Row::Var(v, _) => {
                if let Some(record) = self.row_subst.get(v) {
                    self.apply_row(alloc, record)
                } else {
                    row.clone()
                }
            }

            Row::Extend(label, ty, rest) => {
                let ty = alloc.alloc(self.apply(alloc, ty));
                let rest = alloc.alloc(self.apply_row(alloc, rest));
                Row::Extend(*label, ty, rest)
            }
        }
    }
}
