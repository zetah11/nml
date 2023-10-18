use std::collections::BTreeMap;

use crate::names::{Ident, Name};
use crate::trees::{parsed as i, resolved as o};

use super::{ItemId, Resolver};

impl<'a, 'scratch, 'lit> Resolver<'a, 'scratch, 'lit, '_> {
    pub fn ty(
        &mut self,
        item: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        ty: &i::Type<'_, 'lit>,
    ) -> o::Type<'a, 'lit> {
        let span = ty.span;
        let node = match &ty.node {
            i::TypeNode::Invalid(e) => o::TypeNode::Invalid(*e),
            i::TypeNode::Wildcard => o::TypeNode::Wildcard,

            i::TypeNode::Universal(ident) => {
                // 'a universal types are implicitly defined when used
                if let Some(name) = gen_scope.get(ident) {
                    o::TypeNode::Universal(*name)
                } else {
                    match self.define_type(item, span, i::Affix::Prefix, *ident) {
                        Ok(name) => {
                            gen_scope.insert(*ident, name);
                            o::TypeNode::Universal(name)
                        }

                        Err(e) => o::TypeNode::Invalid(e),
                    }
                }
            }

            i::TypeNode::Function([t, u]) => {
                let t = self.ty(item, gen_scope, t);
                let u = self.ty(item, gen_scope, u);
                o::TypeNode::Function(self.alloc.alloc([t, u]))
            }

            i::TypeNode::Record(fields) => {
                let fields =
                    self.alloc
                        .alloc_slice_fill_iter(fields.iter().map(|(name, name_span, ty)| {
                            let ty = self.ty(item, gen_scope, ty);
                            (*name, *name_span, ty)
                        }));

                o::TypeNode::Record(fields)
            }
        };

        o::Type { node, span }
    }

    pub fn type_pattern(&mut self, item: ItemId, pattern: &i::TypePattern<'lit>) -> o::TypePattern {
        let (affix, ident, span) = pattern.name;
        let name = self.define_type(item, span, affix, ident);
        o::TypePattern { name: (name, span) }
    }
}
