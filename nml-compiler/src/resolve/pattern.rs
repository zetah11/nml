use std::collections::BTreeMap;

use super::{ItemId, Namekind, Namespace, Resolver};
use crate::names::{Ident, Name};
use crate::trees::declared;
use crate::trees::{parsed, resolved};

impl<'a, 'scratch, 'lit> Resolver<'a, 'scratch, 'lit, '_> {
    pub fn name_of(pattern: &resolved::Pattern) -> Option<Name> {
        match &pattern.node {
            resolved::PatternNode::Invalid(_) => None,
            resolved::PatternNode::Wildcard => None,
            resolved::PatternNode::Unit => None,
            resolved::PatternNode::Bind(name) => Some(*name),
            resolved::PatternNode::Constructor(_) => None,
            resolved::PatternNode::Anno(pattern, _) => Resolver::name_of(pattern),
            resolved::PatternNode::Group(pattern) => Resolver::name_of(pattern),
            resolved::PatternNode::Apply([head, body]) => {
                Resolver::name_of(head).or_else(|| Resolver::name_of(body))
            }
        }
    }

    pub fn function_spine(
        &mut self,
        item_id: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        pattern: &'scratch parsed::Pattern<'scratch, 'lit>,
    ) -> declared::Spine<'scratch, 'lit, declared::spined::Pattern<'scratch, 'lit>> {
        match &pattern.node {
            parsed::PatternNode::Anno(pattern, ty) => {
                match self.function_spine(item_id, gen_scope, pattern) {
                    declared::Spine::Fun {
                        head,
                        args,
                        anno: Some(_),
                    } => {
                        let span = ty.span;
                        let e = self
                            .errors
                            .parse_error(span)
                            .multiple_return_type_annotations();
                        let node = parsed::TypeNode::Invalid(e);
                        let anno = self.scratch.alloc(parsed::Type { node, span });
                        declared::Spine::Fun {
                            head,
                            args,
                            anno: Some(anno),
                        }
                    }

                    declared::Spine::Fun {
                        head,
                        args,
                        anno: None,
                    } => {
                        let anno = Some(ty);
                        declared::Spine::Fun { head, args, anno }
                    }

                    declared::Spine::Single(pattern) => {
                        let span = pattern.span + ty.span;
                        let node =
                            declared::spined::PatternNode::Anno(self.scratch.alloc(pattern), ty);

                        declared::Spine::Single(declared::spined::Pattern {
                            node,
                            span,
                            item_id,
                        })
                    }
                }
            }

            parsed::PatternNode::Apply(terms) => self.apply_pattern_run(item_id, gen_scope, terms),
            _ => declared::Spine::Single(self.single_pattern(item_id, gen_scope, pattern)),
        }
    }

    pub fn single_pattern(
        &mut self,
        item_id: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        pattern: &'scratch parsed::Pattern<'scratch, 'lit>,
    ) -> declared::spined::Pattern<'scratch, 'lit> {
        let span = pattern.span;
        let node = match &pattern.node {
            parsed::PatternNode::Invalid(e) => declared::spined::PatternNode::Invalid(*e),
            parsed::PatternNode::Wildcard => declared::spined::PatternNode::Wildcard,
            parsed::PatternNode::Unit => declared::spined::PatternNode::Unit,

            parsed::PatternNode::Bind(name) => {
                if let Some((name, Namekind::Pattern)) = self.lookup_value(&name.1) {
                    declared::spined::PatternNode::Constructor(name)
                } else {
                    declared::spined::PatternNode::Bind(*name)
                }
            }

            parsed::PatternNode::Anno(pattern, ty) => {
                let pattern = self
                    .scratch
                    .alloc(self.single_pattern(item_id, gen_scope, pattern));
                declared::spined::PatternNode::Anno(pattern, ty)
            }

            parsed::PatternNode::Group(pattern) => {
                let pattern = self
                    .scratch
                    .alloc(self.single_pattern(item_id, gen_scope, pattern));
                declared::spined::PatternNode::Group(pattern)
            }

            parsed::PatternNode::Apply(terms) => {
                match self.apply_pattern_run(item_id, gen_scope, terms) {
                    declared::Spine::Single(pattern) => return pattern,
                    declared::Spine::Fun { .. } => {
                        let e = self
                            .errors
                            .parse_error(span)
                            .unexpected_function_definition();
                        declared::spined::PatternNode::Invalid(e)
                    }
                }
            }

            parsed::PatternNode::Constructor(v) => match *v {},
        };

        declared::spined::Pattern {
            node,
            span,
            item_id,
        }
    }

    pub fn pattern(
        &mut self,
        ns: Namespace,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        pattern: &declared::spined::Pattern<'scratch, 'lit>,
    ) -> resolved::Pattern<'a, 'lit> {
        let item_id = pattern.item_id;
        let span = pattern.span;
        let node = match &pattern.node {
            declared::spined::PatternNode::Invalid(e) => resolved::PatternNode::Invalid(*e),
            declared::spined::PatternNode::Wildcard => resolved::PatternNode::Wildcard,
            declared::spined::PatternNode::Unit => resolved::PatternNode::Unit,

            declared::spined::PatternNode::Bind((affix, ident)) => {
                match self.define_name(item_id, span, *affix, *ident, Namekind::Value, ns) {
                    Ok(name) => resolved::PatternNode::Bind(name),
                    Err(e) => resolved::PatternNode::Invalid(e),
                }
            }

            declared::spined::PatternNode::Constructor(name) => {
                resolved::PatternNode::Constructor(*name)
            }

            declared::spined::PatternNode::Anno(pattern, ty) => {
                let pattern = self.alloc.alloc(self.pattern(ns, gen_scope, pattern));
                let ty = self.ty(item_id, gen_scope, ty);
                resolved::PatternNode::Anno(pattern, ty)
            }

            declared::spined::PatternNode::Group(pattern) => {
                let pattern = self.alloc.alloc(self.pattern(ns, gen_scope, pattern));
                resolved::PatternNode::Group(pattern)
            }

            declared::spined::PatternNode::Apply([fun, arg]) => {
                let fun = self.pattern(ns, gen_scope, fun);
                let arg = self.pattern(ns, gen_scope, arg);
                let terms = self.alloc.alloc([fun, arg]);
                resolved::PatternNode::Apply(terms)
            }
        };

        resolved::Pattern { node, span }
    }
}
