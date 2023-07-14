use super::{Solver, TypeVar};
use crate::tyck::memory::Alloc;
use crate::tyck::pretty::Prettifier;
use crate::tyck::tree::RecordRow;
use crate::tyck::{ErrorId, Label, Type};

impl<'a> Solver<'a> {
    pub(super) fn rewrite(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Alloc<'a>,
        label: &Label,
        row: &'a RecordRow<'a>,
        tail: Option<&TypeVar>,
    ) -> (&'a Type<'a>, &'a RecordRow<'a>) {
        match row {
            RecordRow::Empty => {
                let e = ErrorId::new("cannot insert label into empty row");
                (
                    alloc.ty(Type::Invalid(e.clone())),
                    alloc.record(RecordRow::Invalid(e)),
                )
            }

            RecordRow::Extend(old, field, rest) if old == label => (*field, *rest),

            RecordRow::Extend(old, field, rest @ RecordRow::Var(alpha, _)) => {
                // Side condition to ensure termination when records with a
                // common tail but distinct prefix are unified
                if tail == Some(alpha) {
                    let id = ErrorId::new("incompatible records");
                    let e = alloc.record(RecordRow::Invalid(id.clone()));
                    let et = alloc.ty(Type::Invalid(id));
                    self.unify_record(pretty, alloc, rest, e);
                    return (et, e);
                }

                let r = self.fresh_record(alloc);
                let t = self.fresh(alloc);
                let rhs = alloc.record(RecordRow::Extend(label.clone(), t, r));
                self.unify_record(pretty, alloc, rest, rhs);

                let rest = alloc.record(RecordRow::Extend(old.clone(), field, r));
                (t, rest)
            }

            RecordRow::Extend(old, field, rest) => {
                let (label_ty, rest) = self.rewrite(pretty, alloc, label, rest, tail);
                let rest = alloc.record(RecordRow::Extend(old.clone(), field, rest));
                (label_ty, rest)
            }

            RecordRow::Invalid(e) => (alloc.ty(Type::Invalid(e.clone())), row),

            RecordRow::Var(..) | RecordRow::Param(_) => {
                unreachable!("variables are handled by the unification procedure")
            }
        }
    }

    pub(super) fn row_tail<'b>(row: &'b RecordRow) -> Option<&'b TypeVar> {
        match row {
            RecordRow::Var(var, _) => Some(var),
            RecordRow::Extend(_, _, rest) => Self::row_tail(rest),
            RecordRow::Empty | RecordRow::Invalid(_) | RecordRow::Param(_) => None,
        }
    }
}
