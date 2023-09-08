use crate::trees::{parsed as i, resolved as o};

use super::Resolver;

impl<'a, 'lit> Resolver<'a, 'lit, '_> {
    pub fn ty(&mut self, ty: &i::Type<'_, 'lit>) -> o::Type<'a, 'lit> {
        let span = ty.span;
        let node = match &ty.node {
            i::TypeNode::Invalid(e) => o::TypeNode::Invalid(*e),
            i::TypeNode::Hole => o::TypeNode::Hole,
            i::TypeNode::Function([t, u]) => {
                let t = self.ty(t);
                let u = self.ty(u);
                o::TypeNode::Function(self.alloc.alloc([t, u]))
            }
        };

        o::Type { node, span }
    }
}
