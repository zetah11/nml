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

            parsed::PatternNode::Apply(terms) => {
                return self.apply_pattern_run(item, gen_scope, terms);
            }

            parsed::PatternNode::Anno(pattern, ty) => {
                let pattern = self.alloc.alloc(self.pattern(item, gen_scope, pattern));
                let ty = self.ty(item, gen_scope, ty);
                resolved::PatternNode::Anno(pattern, ty)
            }

            parsed::PatternNode::Group(pattern) => {
                let pattern = self.alloc.alloc(self.pattern(item, gen_scope, pattern));
                resolved::PatternNode::Group(pattern)
            }

            parsed::PatternNode::Bind(x) => match *x {},
            parsed::PatternNode::Constructor(x) => match *x {},
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
