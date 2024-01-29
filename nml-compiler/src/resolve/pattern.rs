use std::collections::{BTreeMap, BTreeSet};

use super::{ItemId, Namekind, Namespace, Resolver};
use crate::errors::ErrorId;
use crate::names::{Ident, Name};
use crate::source::Span;
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

            resolved::PatternNode::Apply([a, b])
            | resolved::PatternNode::Or([a, b])
            | resolved::PatternNode::And([a, b]) => {
                Resolver::name_of(a).or_else(|| Resolver::name_of(b))
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

            parsed::PatternNode::Or([a, b]) => {
                let a = self.single_pattern(item_id, gen_scope, a);
                let b = self.single_pattern(item_id, gen_scope, b);
                let terms = self.scratch.alloc([a, b]);
                declared::spined::PatternNode::Or(terms)
            }

            parsed::PatternNode::And([a, b]) => {
                let a = self.single_pattern(item_id, gen_scope, a);
                let b = self.single_pattern(item_id, gen_scope, b);
                let terms = self.scratch.alloc([a, b]);
                declared::spined::PatternNode::And(terms)
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
        let (pattern, _) = self.declare_pattern(ns, gen_scope, pattern, &BTreeMap::new());
        pattern
    }

    /// Declare (and hence resolve) the names bound by `pattern`.
    ///
    /// The map `known` is used with or-patterns: when the right-hand side of an
    /// or-pattern declares a name, that name _should_ have been declared by the
    /// left-hand side.  To ensure both names resolve to the same [`Name`], the
    /// `known` map "caches" identifiers between or-patterns.
    fn declare_pattern(
        &mut self,
        ns: Namespace,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        pattern: &declared::spined::Pattern<'scratch, 'lit>,
        known: &BTreeMap<Ident<'lit>, Name>,
    ) -> (resolved::Pattern<'a, 'lit>, BTreeMap<Ident<'lit>, Name>) {
        let item_id = pattern.item_id;
        let span = pattern.span;
        let (node, names) = match &pattern.node {
            declared::spined::PatternNode::Invalid(e) => {
                (resolved::PatternNode::Invalid(*e), BTreeMap::new())
            }
            declared::spined::PatternNode::Wildcard => {
                (resolved::PatternNode::Wildcard, BTreeMap::new())
            }
            declared::spined::PatternNode::Unit => (resolved::PatternNode::Unit, BTreeMap::new()),

            declared::spined::PatternNode::Bind((affix, ident)) => {
                if let Some(name) = known.get(ident) {
                    let names = BTreeMap::from([(*ident, *name)]);

                    let previous_affix = self
                        .affii
                        .get(name)
                        .expect("all declared names have an affix");
                    let previous_span = self
                        .spans
                        .get(name)
                        .expect("all declared names have a span");

                    if previous_affix != affix {
                        let e = self.errors.name_error(span).affii_disagree(*previous_span);
                        (resolved::PatternNode::Invalid(e), names)
                    } else {
                        (resolved::PatternNode::Bind(*name), names)
                    }
                } else {
                    match self.define_name(item_id, span, *affix, *ident, Namekind::Value, ns) {
                        Ok(name) => {
                            let names = BTreeMap::from([(*ident, name)]);
                            (resolved::PatternNode::Bind(name), names)
                        }

                        Err(e) => (resolved::PatternNode::Invalid(e), BTreeMap::new()),
                    }
                }
            }

            declared::spined::PatternNode::Constructor(name) => {
                (resolved::PatternNode::Constructor(*name), BTreeMap::new())
            }

            declared::spined::PatternNode::Anno(pattern, ty) => {
                let (pattern, names) = self.declare_pattern(ns, gen_scope, pattern, known);
                let pattern = self.alloc.alloc(pattern);
                let ty = self.resolve_type(item_id, gen_scope, ty);
                (resolved::PatternNode::Anno(pattern, ty), names)
            }

            declared::spined::PatternNode::Group(pattern) => {
                let (pattern, names) = self.declare_pattern(ns, gen_scope, pattern, known);
                let pattern = self.alloc.alloc(pattern);
                (resolved::PatternNode::Group(pattern), names)
            }

            declared::spined::PatternNode::Apply([fun, arg]) => {
                let (fun, names1) = self.declare_pattern(ns, gen_scope, fun, known);
                let (arg, names2) = self.declare_pattern(ns, gen_scope, arg, known);
                let terms = self.alloc.alloc([fun, arg]);

                let mut names = names1;
                for (ident, name) in names2 {
                    let prev = names.insert(ident, name);
                    debug_assert!(prev.is_none());
                }

                (resolved::PatternNode::Apply(terms), names)
            }

            declared::spined::PatternNode::Or([a, b]) => {
                let (a, a_names) = self.declare_pattern(ns, gen_scope, a, known);

                // Known names in `b` are the already known names _and_ whatever
                // names `a` declares
                let mut b_known = known.clone();
                for (ident, name) in a_names.iter() {
                    let prev = b_known.insert(*ident, *name);
                    debug_assert!(prev.is_none());
                }

                let (b, b_names) = self.declare_pattern(ns, gen_scope, b, &b_known);

                // Ensure `a` and `b` declare the exact same set of names
                let declared_in_a: BTreeSet<&Ident> = a_names.keys().collect();
                let declared_in_b: BTreeSet<&Ident> = b_names.keys().collect();
                let difference: Vec<_> =
                    declared_in_a.symmetric_difference(&declared_in_b).collect();

                if difference.is_empty() {
                    let terms = self.alloc.alloc([a, b]);
                    (resolved::PatternNode::Or(terms), a_names)
                } else {
                    let names = difference
                        .into_iter()
                        .map(|name| self.names.get_ident(name));

                    let e = self.errors.name_error(span).or_patterns_disagree(names);

                    let names: BTreeMap<_, _> = a_names.into_iter().chain(b_names).collect();
                    let node = self.poison(span, e, names.values().copied());
                    return (node, names);
                }
            }

            declared::spined::PatternNode::And([a, b]) => {
                let (a, mut a_names) = self.declare_pattern(ns, gen_scope, a, known);
                let (b, b_names) = self.declare_pattern(ns, gen_scope, b, known);

                for (ident, name) in b_names {
                    let prev = a_names.insert(ident, name);
                    assert!(prev.is_none());
                }

                let terms = self.alloc.alloc([a, b]);
                (resolved::PatternNode::And(terms), a_names)
            }
        };

        (resolved::Pattern { node, span }, names)
    }

    /// Apply an error node to a set of names.  This makes sure that the given
    /// names exist _somewhere_, while keeping them erroneous.
    fn poison(
        &self,
        span: Span,
        e: ErrorId,
        names: impl IntoIterator<Item = Name>,
    ) -> resolved::Pattern<'a, 'lit> {
        let node = resolved::PatternNode::Invalid(e);
        let mut pattern = resolved::Pattern { node, span };

        for name in names {
            let node = resolved::PatternNode::Bind(name);
            let arg = resolved::Pattern { node, span };
            let terms = self.alloc.alloc([pattern, arg]);
            let node = resolved::PatternNode::Apply(terms);
            pattern = resolved::Pattern { node, span };
        }

        pattern
    }
}
