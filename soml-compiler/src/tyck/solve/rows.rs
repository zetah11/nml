use typed_arena::Arena;

use super::Solver;
use crate::tyck::{ErrorId, Label, Type};

impl<'a> Solver<'a> {
    pub(super) fn rewrite(
        &mut self,
        alloc: &'a Arena<Type<'a>>,
        ty: &'a Type<'a>,
        label: &Label,
    ) -> (&'a Type<'a>, &'a Type<'a>) {
        match ty {
            Type::Empty => (
                alloc.alloc(Type::Invalid(ErrorId::new(
                    "cannot insert label into empty row",
                ))),
                alloc.alloc(Type::Invalid(ErrorId::new(
                    "cannot insert label into empty row",
                ))),
            ),

            Type::Extend(old, field, rest) if old == label => (*field, *rest),

            Type::Extend(old, field, rest @ Type::Var(..)) => {
                let beta = self.fresh(alloc);
                let gamma = self.fresh(alloc);
                let rhs = alloc.alloc(Type::Extend(label.clone(), gamma, beta));
                self.unify(alloc, rest, rhs);

                let rest = alloc.alloc(Type::Extend(old.clone(), field, beta));
                (gamma, rest)
            }

            Type::Extend(old, field, rest) => {
                let (label_ty, rest) = self.rewrite(alloc, rest, label);
                let rest = alloc.alloc(Type::Extend(old.clone(), field, rest));
                (label_ty, rest)
            }

            Type::Invalid(_) => (ty, ty),

            _ => unreachable!("row rewriting only happens on record types"),
        }
    }
}
