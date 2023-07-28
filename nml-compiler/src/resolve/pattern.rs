use crate::names::Label;
use crate::trees::resolved::ItemId;
use crate::trees::{parsed, resolved};

use super::Resolver;

impl<'a> Resolver<'a, '_> {
    pub fn pattern(
        &mut self,
        item: ItemId,
        pattern: &parsed::Pattern,
    ) -> &'a resolved::Pattern<'a> {
        let span = pattern.span;
        let node = match &pattern.node {
            parsed::PatternNode::Invalid(e) => resolved::PatternNode::Invalid(*e),
            parsed::PatternNode::Wildcard => resolved::PatternNode::Wildcard,
            parsed::PatternNode::Unit => resolved::PatternNode::Unit,

            parsed::PatternNode::Small(name) => match self.define_value(item, span, *name) {
                Ok(name) => resolved::PatternNode::Bind(name),
                Err(e) => resolved::PatternNode::Invalid(e),
            },

            parsed::PatternNode::Big(name) => {
                if self.lookup_value(name).is_some() {
                    todo!("non-anonymous variant")
                } else {
                    let name = self.names.get_ident(name);
                    resolved::PatternNode::Invalid(
                        self.errors.name_error(span).unapplied_anonymous_variant(name),
                    )
                }
            }

            parsed::PatternNode::Apply(
                parsed::Pattern { node: parsed::PatternNode::Big(name), .. },
                arg,
            ) if self.lookup_value(name).is_none() => {
                let label = Label(*name);
                let arg = self.pattern(item, arg);
                resolved::PatternNode::Deconstruct(label, arg)
            }

            parsed::PatternNode::Apply(fun, arg) => {
                let fun = self.pattern(item, fun);
                let arg = self.pattern(item, arg);
                resolved::PatternNode::Apply(fun, arg)
            }
        };

        self.alloc.alloc(resolved::Pattern { node, span })
    }
}