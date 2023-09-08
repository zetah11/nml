use super::{types as o, Checker};
use crate::trees::resolved as i;

impl<'a, 'err, 'ids, 'p> Checker<'a, 'err, 'ids, 'p> {
    /// Lower a type expression into its semantic equivalent.
    pub(super) fn lower(&mut self, ty: &i::Type<'_, 'ids>) -> &'a o::Type<'a> {
        let ty = match &ty.node {
            i::TypeNode::Invalid(e) => o::Type::Invalid(*e),
            i::TypeNode::Hole => return self.fresh(),
            i::TypeNode::Function([t, u]) => {
                let t = self.lower(t);
                let u = self.lower(u);
                o::Type::Fun(t, u)
            }
        };

        self.alloc.alloc(ty)
    }
}
