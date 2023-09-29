use std::collections::BTreeMap;

use crate::names::{Ident, Name};
use crate::trees::{parsed, resolved};

use super::{ItemId, Resolver};

impl<'a, 'lit> Resolver<'a, 'lit, '_> {
    pub fn pattern(
        &mut self,
        item: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        pattern: &parsed::Pattern<'_, 'lit>,
    ) -> resolved::Pattern<'a, 'lit> {
        let span = pattern.span;
        let node = match &pattern.node {
            parsed::PatternNode::Invalid(e) => resolved::PatternNode::Invalid(*e),
            parsed::PatternNode::Wildcard => resolved::PatternNode::Wildcard,
            parsed::PatternNode::Unit => resolved::PatternNode::Unit,

            parsed::PatternNode::Name((affix, name)) => {
                match self.define_value(item, span, *affix, *name) {
                    Ok(name) => resolved::PatternNode::Bind(name),
                    Err(e) => resolved::PatternNode::Invalid(e),
                }
            }

            parsed::PatternNode::Big((_, name)) => {
                if self.lookup_value(name).is_some() {
                    todo!("non-anonymous variant")
                } else {
                    let name = self.names.get_ident(name);
                    resolved::PatternNode::Invalid(
                        self.errors
                            .name_error(span)
                            .unapplied_anonymous_variant(name),
                    )
                }
            }

            parsed::PatternNode::Apply([fun, arg]) => {
                let fun = self.pattern(item, gen_scope, fun);
                let arg = self.pattern(item, gen_scope, arg);
                resolved::PatternNode::Apply(self.alloc.alloc([fun, arg]))
            }

            parsed::PatternNode::Anno(pattern, ty) => {
                let pattern = self.alloc.alloc(self.pattern(item, gen_scope, pattern));
                let ty = self.ty(item, gen_scope, ty);
                resolved::PatternNode::Anno(pattern, ty)
            }

            parsed::PatternNode::Bind(x) => match *x {},
            parsed::PatternNode::Named(x) => match *x {},
        };

        resolved::Pattern { node, span }
    }

    pub fn name_of(pattern: &resolved::Pattern) -> Option<Name> {
        if let resolved::PatternNode::Bind(name) = &pattern.node {
            Some(*name)
        } else {
            None
        }
    }
}
