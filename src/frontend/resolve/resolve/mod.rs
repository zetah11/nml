use std::collections::BTreeMap;

use crate::frontend::errors::ErrorId;
use crate::frontend::names::Name;
use crate::frontend::resolve::Namespace;
use crate::frontend::trees::{declared, parsed, resolved};

use super::{ItemId, Resolver};

impl<'a, 'scratch, 'lit, 'err> Resolver<'a, 'scratch, 'lit, 'err> {
    pub(super) fn resolve_item(
        &mut self,
        item: declared::Item<'a, 'scratch, 'lit>,
    ) -> resolved::Item<'a, 'lit> {
        let id = item.id;
        let span = item.span;
        let node = match item.node {
            declared::ItemNode::Invalid(e) => resolved::ItemNode::Invalid(e),
            declared::ItemNode::Let(spine, expr, mut this_scope) => {
                let (pattern, expr) = match spine {
                    declared::Spine::Single(pattern) => {
                        let expr = self.expr(id, &mut this_scope, expr);
                        (pattern, expr)
                    }

                    declared::Spine::Fun { head, args, anno } => {
                        let expr = if let Some(ty) = anno {
                            let span = expr.span + ty.span;
                            let node = parsed::ExprNode::Anno(expr, *ty);
                            self.scratch.alloc(parsed::Expr { node, span })
                        } else {
                            expr
                        };

                        let body = self.lambda(id, &mut this_scope, &args, expr);

                        (head, body)
                    }
                };

                resolved::ItemNode::Let(
                    pattern,
                    expr,
                    self.alloc.alloc_slice_fill_iter(this_scope.into_values()),
                )
            }

            declared::ItemNode::Data(spine, body) => {
                let (pattern, body) = match spine {
                    declared::Spine::Single(pattern) => {
                        let pattern = resolved::DataPattern {
                            name: self.resolve_data_pattern_name(&pattern),
                            args: self.alloc.alloc([]),
                        };

                        let body = self.resolve_data(id, body);
                        (pattern, body)
                    }

                    declared::Spine::Fun { head, args, anno } => {
                        let name = if let Some(anno) = anno {
                            Err(self
                                .errors
                                .parse_error(anno.span)
                                .kind_annotations_unsupported())
                        } else {
                            self.resolve_data_pattern_name(&head)
                        };

                        let (args, body) = self.scope(name.ok(), |this| {
                            let args =
                                this.alloc
                                    .alloc_slice_fill_iter(args.into_iter().map(|pattern| {
                                        let mut gen_scope = BTreeMap::new();
                                        let pattern =
                                            this.pattern(Namespace::Type, &mut gen_scope, &pattern);
                                        this.resolve_data_pattern_name(&pattern)
                                    }));

                            this.explicit_universals
                                .extend(args.iter().flat_map(|name| name.ok()));

                            let body = this.resolve_data(id, body);
                            (&*args, body)
                        });

                        let pattern = resolved::DataPattern { name, args };
                        (pattern, body)
                    }
                };

                resolved::ItemNode::Data(pattern, body)
            }
        };

        resolved::Item { id, node, span }
    }

    fn resolve_data_pattern_name(
        &mut self,
        pattern: &resolved::Pattern<'a, 'lit>,
    ) -> Result<Name, ErrorId> {
        match &pattern.node {
            resolved::PatternNode::Invalid(e) => Err(*e),
            resolved::PatternNode::Bind(name) => Ok(*name),
            resolved::PatternNode::Group(pattern) => self.resolve_data_pattern_name(pattern),

            resolved::PatternNode::Wildcard
            | resolved::PatternNode::Unit
            | resolved::PatternNode::Constructor(_)
            | resolved::PatternNode::Anno(_, _)
            | resolved::PatternNode::Apply(_)
            | resolved::PatternNode::Or(_)
            | resolved::PatternNode::And(_) => {
                let span = pattern.span;
                let e = self.errors.parse_error(span).expected_name();
                Err(e)
            }
        }
    }

    fn resolve_data(
        &mut self,
        item: ItemId,
        data: declared::patterns::Data<'scratch, 'lit>,
    ) -> resolved::Data<'a, 'lit> {
        let span = data.span;
        let node = match data.node {
            declared::patterns::DataNode::Invalid(e) => resolved::DataNode::Invalid(e),
            declared::patterns::DataNode::Sum(ctors) => {
                let ctors = self.alloc.alloc_slice_fill_iter(
                    ctors
                        .iter()
                        .map(|ctor| self.resolve_constructor(item, ctor)),
                );

                resolved::DataNode::Sum(ctors)
            }
        };

        resolved::Data { node, span }
    }

    fn resolve_constructor(
        &mut self,
        item: ItemId,
        ctor: &'scratch declared::patterns::Constructor<'scratch, 'lit>,
    ) -> resolved::Constructor<'a, 'lit> {
        let span = ctor.span;
        let node = match &ctor.node {
            declared::patterns::ConstructorNode::Invalid(e) => {
                resolved::ConstructorNode::Invalid(*e)
            }

            declared::patterns::ConstructorNode::Constructor(name, params) => {
                let params = self.alloc.alloc_slice_fill_iter(params.iter().map(|ty| {
                    let mut gen_scope = BTreeMap::new();
                    let ty = self.resolve_type(item, &mut gen_scope, ty);

                    if !gen_scope.is_empty() {
                        let span = ty.span;
                        let e = self.errors.name_error(span).implicit_type_var_in_data();
                        let node = resolved::TypeNode::Invalid(e);
                        resolved::Type { node, span }
                    } else {
                        ty
                    }
                }));

                resolved::ConstructorNode::Constructor(*name, params)
            }
        };

        resolved::Constructor { node, span }
    }
}
