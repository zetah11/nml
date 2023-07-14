use super::Solver;
use crate::tyck::memory::Alloc;
use crate::tyck::pretty::Prettifier;
use crate::tyck::tree::RecordRow;
use crate::tyck::{ErrorId, Label, Type};

impl<'a> Solver<'a> {
    pub(super) fn rewrite(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Alloc<'a>,
        row: &'a RecordRow<'a>,
        label: &Label,
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

            RecordRow::Extend(old, field, rest @ RecordRow::Var(..)) => {
                let r = self.fresh_record(alloc);
                let t = self.fresh(alloc);
                let rhs = alloc.record(RecordRow::Extend(label.clone(), t, r));
                self.unify_record(pretty, alloc, rest, rhs);

                let rest = alloc.record(RecordRow::Extend(old.clone(), field, r));
                (t, rest)
            }

            RecordRow::Extend(old, field, rest) => {
                let (label_ty, rest) = self.rewrite(pretty, alloc, rest, label);
                let rest = alloc.record(RecordRow::Extend(old.clone(), field, rest));
                (label_ty, rest)
            }

            RecordRow::Invalid(e) => (alloc.ty(Type::Invalid(e.clone())), row),

            RecordRow::Var(..) | RecordRow::Param(_) => {
                unreachable!("variables are handled by the unification procedure")
            }
        }
    }
}
