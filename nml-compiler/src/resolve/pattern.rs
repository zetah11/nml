use std::collections::BTreeMap;

use super::{ItemId, Resolver};
use crate::names::{Ident, Name};
use crate::trees::declared::Spine;
use crate::trees::{parsed, resolved};

impl<'a, 'lit> Resolver<'a, 'lit, '_> {
    pub fn function_spine(
        &mut self,
        item_id: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        pattern: &parsed::Pattern<'_, 'lit>,
    ) -> Spine<'a, 'lit> {
        match &pattern.node {
            parsed::PatternNode::Group(pattern) => self.function_spine(item_id, gen_scope, pattern),
            parsed::PatternNode::Apply(terms) => self.apply_pattern_run(item_id, gen_scope, terms),
            _ => Spine::Single(self.single_pattern(item_id, gen_scope, pattern)),
        }
    }

    pub fn single_pattern(
        &mut self,
        item_id: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        pattern: &parsed::Pattern<'_, 'lit>,
    ) -> resolved::Pattern<'a, 'lit> {
        let span = pattern.span;
        let node = match &pattern.node {
            parsed::PatternNode::Invalid(e) => resolved::PatternNode::Invalid(*e),
            parsed::PatternNode::Wildcard => resolved::PatternNode::Wildcard,
            parsed::PatternNode::Unit => resolved::PatternNode::Unit,

            parsed::PatternNode::Name((affix, name)) => {
                match self.define_value(item_id, span, *affix, *name) {
                    Ok(name) => resolved::PatternNode::Bind(name),
                    Err(e) => resolved::PatternNode::Invalid(e),
                }
            }

            parsed::PatternNode::Apply(terms) => {
                match self.apply_pattern_run(item_id, gen_scope, terms) {
                    Spine::Single(pattern) => return pattern,
                    Spine::Fun { .. } => {
                        let e = self
                            .errors
                            .parse_error(span)
                            .unexpected_function_definition();
                        resolved::PatternNode::Invalid(e)
                    }
                }
            }

            parsed::PatternNode::Anno(pattern, ty) => {
                let pattern = self
                    .alloc
                    .alloc(self.single_pattern(item_id, gen_scope, pattern));
                let ty = self.ty(item_id, gen_scope, ty);
                resolved::PatternNode::Anno(pattern, ty)
            }

            parsed::PatternNode::Group(pattern) => {
                let pattern = self
                    .alloc
                    .alloc(self.single_pattern(item_id, gen_scope, pattern));
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
