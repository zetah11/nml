use super::{Solver, TypeVar};
use crate::names::Label;
use crate::tyck::memory::Alloc;
use crate::tyck::tree::RecordRow;
use crate::tyck::{Reporting, Type};

impl<'a> Solver<'a> {
    pub(super) fn rewrite(
        &mut self,
        reporting: &mut Reporting,
        alloc: &'a Alloc<'a>,
        label: &Label,
        row: &'a RecordRow<'a>,
        tail: Option<&TypeVar>,
    ) -> (&'a Type<'a>, &'a RecordRow<'a>) {
        match row {
            RecordRow::Empty => {
                let e = reporting
                    .errors
                    .type_error(reporting.at)
                    .no_such_label(reporting.pretty.label(label));

                (
                    alloc.ty(Type::Invalid(e)),
                    alloc.record(RecordRow::Invalid(e)),
                )
            }

            RecordRow::Extend(old, field, rest) if old == label => (*field, *rest),

            RecordRow::Extend(old, field, rest @ RecordRow::Var(alpha, _)) => {
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
                    let e = alloc.record(RecordRow::Invalid(id));
                    let et = alloc.ty(Type::Invalid(id));
                    self.unify_record(reporting, alloc, rest, e);
                    return (et, e);
                }

                let r = self.fresh_record(alloc);
                let t = self.fresh(alloc);
                let rhs = alloc.record(RecordRow::Extend(*label, t, r));
                self.unify_record(reporting, alloc, rest, rhs);

                let rest = alloc.record(RecordRow::Extend(*old, field, r));
                (t, rest)
            }

            RecordRow::Extend(old, field, rest) => {
                let (label_ty, rest) = self.rewrite(reporting, alloc, label, rest, tail);
                let rest = alloc.record(RecordRow::Extend(*old, field, rest));
                (label_ty, rest)
            }

            RecordRow::Invalid(e) => (alloc.ty(Type::Invalid(*e)), row),

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
