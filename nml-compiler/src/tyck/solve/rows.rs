use bumpalo::Bump;

use super::{Solver, TypeVar};
use crate::names::Label;
use crate::tyck::types::Row;
use crate::tyck::{Reporting, Type};

impl<'a> Solver<'a> {
    pub(super) fn rewrite(
        &mut self,
        reporting: &mut Reporting,
        alloc: &'a Bump,
        label: &Label,
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

    pub(super) fn row_tail<'b>(row: &'b Row) -> Option<&'b TypeVar> {
        match row {
            Row::Var(var, _) => Some(var),
            Row::Extend(_, _, rest) => Self::row_tail(rest),
            Row::Empty | Row::Invalid(_) | Row::Param(_) => None,
        }
    }
}
