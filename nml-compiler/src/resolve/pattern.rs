use std::collections::BTreeMap;

use super::{ItemId, Resolver, ValueNamespace};
use crate::names::{Ident, Name};
use crate::trees::declared;
use crate::trees::{parsed, resolved};

impl<'a, 'scratch, 'lit> Resolver<'a, 'scratch, 'lit, '_> {
    pub fn name_of(pattern: &resolved::Pattern) -> Option<Name> {
        if let resolved::PatternNode::Bind(name) = &pattern.node {
            Some(*name)
        } else {
            None
        }
    }

    pub fn function_spine(
        &mut self,
        item_id: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        pattern: &'scratch parsed::Pattern<'scratch, 'lit>,
    ) -> declared::Spine<'scratch, 'lit, declared::SpinedPattern<'scratch, 'lit>> {
        match &pattern.node {
            parsed::PatternNode::Anno(_, _) => todo!(),
            parsed::PatternNode::Apply(terms) => self.apply_pattern_run(item_id, gen_scope, terms),
            _ => declared::Spine::Single(self.single_pattern(item_id, gen_scope, pattern)),
        }
    }

    pub fn single_pattern(
        &mut self,
        item_id: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        pattern: &'scratch parsed::Pattern<'scratch, 'lit>,
    ) -> declared::SpinedPattern<'scratch, 'lit> {
        let span = pattern.span;
        let node = match &pattern.node {
            parsed::PatternNode::Invalid(e) => declared::SpinedPatternNode::Invalid(*e),
            parsed::PatternNode::Wildcard => declared::SpinedPatternNode::Wildcard,
            parsed::PatternNode::Unit => declared::SpinedPatternNode::Unit,

            parsed::PatternNode::Bind(name) => {
                if let Some((name, ValueNamespace::Pattern)) = self.lookup_value(&name.1) {
                    declared::SpinedPatternNode::Constructor(name)
                } else {
                    declared::SpinedPatternNode::Bind(*name)
                }
            }

            parsed::PatternNode::Anno(pattern, ty) => {
                let pattern = self
                    .scratch
                    .alloc(self.single_pattern(item_id, gen_scope, pattern));
                declared::SpinedPatternNode::Anno(pattern, ty)
            }

            parsed::PatternNode::Group(pattern) => {
                let pattern = self
                    .scratch
                    .alloc(self.single_pattern(item_id, gen_scope, pattern));
                declared::SpinedPatternNode::Group(pattern)
            }

            parsed::PatternNode::Apply(terms) => {
                match self.apply_pattern_run(item_id, gen_scope, terms) {
                    declared::Spine::Single(pattern) => return pattern,
                    declared::Spine::Fun { .. } => {
                        let e = self
                            .errors
                            .parse_error(span)
                            .unexpected_function_definition();
                        declared::SpinedPatternNode::Invalid(e)
                    }
                }
            }

            parsed::PatternNode::Constructor(v) => match *v {},
        };

        declared::SpinedPattern {
            node,
            span,
            item_id,
        }
    }

    pub fn pattern(
        &mut self,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        pattern: &declared::SpinedPattern<'scratch, 'lit>,
    ) -> resolved::Pattern<'a, 'lit> {
        let item_id = pattern.item_id;
        let span = pattern.span;
        let node = match &pattern.node {
            declared::SpinedPatternNode::Invalid(e) => resolved::PatternNode::Invalid(*e),
            declared::SpinedPatternNode::Wildcard => resolved::PatternNode::Wildcard,
            declared::SpinedPatternNode::Unit => resolved::PatternNode::Unit,

            declared::SpinedPatternNode::Bind((affix, ident)) => {
                match self.define_value(item_id, span, *affix, *ident, ValueNamespace::Value) {
                    Ok(name) => resolved::PatternNode::Bind(name),
                    Err(e) => resolved::PatternNode::Invalid(e),
                }
            }

            declared::SpinedPatternNode::Constructor(name) => {
                resolved::PatternNode::Constructor(*name)
            }

            declared::SpinedPatternNode::Anno(pattern, ty) => {
                let pattern = self.alloc.alloc(self.pattern(gen_scope, pattern));
                let ty = self.ty(item_id, gen_scope, ty);
                resolved::PatternNode::Anno(pattern, ty)
            }

            declared::SpinedPatternNode::Group(pattern) => {
                let pattern = self.alloc.alloc(self.pattern(gen_scope, pattern));
                resolved::PatternNode::Group(pattern)
            }

            declared::SpinedPatternNode::Apply([fun, arg]) => {
                let fun = self.pattern(gen_scope, fun);
                let arg = self.pattern(gen_scope, arg);
                let terms = self.alloc.alloc([fun, arg]);
                resolved::PatternNode::Apply(terms)
            }
        };

        resolved::Pattern { node, span }
    }
}
