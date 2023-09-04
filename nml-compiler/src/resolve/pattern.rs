use crate::names::{Label, Name};
use crate::trees::{parsed, resolved};

use super::{ItemId, Resolver};

impl<'a> Resolver<'a, '_> {
    pub fn pattern<'lit>(
        &mut self,
        item: ItemId,
        pattern: &parsed::Pattern<'_, 'lit>,
    ) -> resolved::Pattern<'a, 'lit> {
        let span = pattern.span;
        let node = match &pattern.node {
            parsed::PatternNode::Invalid(e) => resolved::PatternNode::Invalid(*e),
            parsed::PatternNode::Wildcard => resolved::PatternNode::Wildcard,
            parsed::PatternNode::Unit => resolved::PatternNode::Unit,

            parsed::PatternNode::Small((affix, name)) => {
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

            parsed::PatternNode::Apply(
                [parsed::Pattern {
                    node: parsed::PatternNode::Big((_, name)),
                    ..
                }, arg],
            ) if self.lookup_value(name).is_none() => {
                let label = Label(*name);
                let arg = self.pattern(item, arg);
                let arg = self.alloc.alloc(arg);
                resolved::PatternNode::Deconstruct(label, arg)
            }

            parsed::PatternNode::Apply([fun, arg]) => {
                let fun = self.pattern(item, fun);
                let arg = self.pattern(item, arg);
                resolved::PatternNode::Apply(self.alloc.alloc([fun, arg]))
            }

            parsed::PatternNode::Bind(x) => match *x {},
            parsed::PatternNode::Named(x) => match *x {},
            parsed::PatternNode::Deconstruct(x, _) => match *x {},
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
