use super::{types as o, Checker};
use crate::trees::resolved as i;

impl<'a, 'err, 'ids, 'p> Checker<'a, 'err, 'ids, 'p> {
    /// Lower a type expression into its semantic equivalent.
    pub(super) fn lower(&mut self, ty: &i::Type<'_, 'ids>) -> &'a o::Type<'a> {
        let ty = match &ty.node {
            i::TypeNode::Invalid(e) => o::Type::Invalid(*e),
            i::TypeNode::Wildcard => return self.fresh(),

            i::TypeNode::Named(name) => o::Type::Named(*name, self.alloc.alloc([])),

            i::TypeNode::Universal(name) => o::Type::Param(o::Generic::Ticked(*name)),

            i::TypeNode::Function([t, u]) => {
                let t = self.lower(t);
                let u = self.lower(u);
                o::Type::Fun(t, u)
            }

            i::TypeNode::Record(fields) => {
                let mut row: &_ = self.alloc.alloc(o::Row::Empty);

                for (field_name, _, ty) in fields.iter() {
                    let ty = self.lower(ty);
                    if let Ok(field_name) = field_name {
                        row = self.alloc.alloc(o::Row::Extend(*field_name, ty, row));
                    }
                }

                o::Type::Record(self.alloc.alloc(row))
            }
        };

        self.alloc.alloc(ty)
    }
}
