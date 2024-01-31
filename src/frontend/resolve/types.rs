use std::collections::BTreeMap;

use crate::frontend::names::{Ident, Name};
use crate::frontend::trees::{parsed as i, resolved as o};

use super::{ItemId, Resolver};

impl<'a, 'scratch, 'src> Resolver<'a, 'scratch, 'src, '_> {
    pub fn resolve_type(
        &mut self,
        item_id: ItemId,
        gen_scope: &mut BTreeMap<Ident<'src>, Name>,
        ty: &'scratch i::Type<'scratch, 'src>,
    ) -> o::Type<'a, 'src> {
        let span = ty.span;
        let node = match &ty.node {
            i::TypeNode::Invalid(e) => o::TypeNode::Invalid(*e),
            i::TypeNode::Wildcard => o::TypeNode::Wildcard,

            i::TypeNode::Named(name) => {
                if let Some(name) = self.lookup_type(name) {
                    if self.explicit_universals.contains(&name) {
                        o::TypeNode::Universal(name)
                    } else {
                        o::TypeNode::Named(name)
                    }
                } else {
                    let name = self.names.get_ident(name);
                    o::TypeNode::Invalid(self.errors.name_error(span).unknown_name(name))
                }
            }

            i::TypeNode::Universal(ident) => {
                // 'a universal types are implicitly defined when used
                if let Some(name) = gen_scope.get(ident) {
                    o::TypeNode::Universal(*name)
                } else {
                    match self.define_type(item_id, span, i::Affix::Prefix, *ident) {
                        Ok(name) => {
                            gen_scope.insert(*ident, name);
                            o::TypeNode::Universal(name)
                        }

                        Err(e) => o::TypeNode::Invalid(e),
                    }
                }
            }

            i::TypeNode::Function([t, u]) => {
                let t = self.resolve_type(item_id, gen_scope, t);
                let u = self.resolve_type(item_id, gen_scope, u);
                o::TypeNode::Function(self.alloc.alloc([t, u]))
            }

            i::TypeNode::Record(fields) => {
                let fields =
                    self.alloc
                        .alloc_slice_fill_iter(fields.iter().map(|(name, name_span, ty)| {
                            let ty = self.resolve_type(item_id, gen_scope, ty);
                            (*name, *name_span, ty)
                        }));

                o::TypeNode::Record(fields)
            }

            i::TypeNode::Group(ty) => {
                let ty = self.resolve_type(item_id, gen_scope, ty);
                o::TypeNode::Group(self.alloc.alloc(ty))
            }

            i::TypeNode::Apply(run) => return self.apply_type_run(item_id, gen_scope, run),
        };

        o::Type { node, span }
    }
}
