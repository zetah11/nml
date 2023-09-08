use crate::trees::{parsed as i, resolved as o};

use super::Resolver;

impl<'a, 'lit> Resolver<'a, 'lit, '_> {
    pub fn ty(&mut self, ty: &i::Type<'_, 'lit>) -> o::Type<'a, 'lit> {
        let span = ty.span;
        let node = match &ty.node {
            i::TypeNode::Invalid(e) => o::TypeNode::Invalid(*e),
            i::TypeNode::Wildcard => o::TypeNode::Wildcard,
            i::TypeNode::Function([t, u]) => {
                let t = self.ty(t);
                let u = self.ty(u);
                o::TypeNode::Function(self.alloc.alloc([t, u]))
            }

            i::TypeNode::Record(fields) => {
                let fields =
                    self.alloc
                        .alloc_slice_fill_iter(fields.iter().map(|(name, name_span, ty)| {
                            let ty = self.ty(ty);
                            (*name, *name_span, ty)
                        }));

                o::TypeNode::Record(fields)
            }
        };

        o::Type { node, span }
    }
}
