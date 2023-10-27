use std::collections::BTreeMap;

use crate::trees::{declared, parsed, resolved};

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
                let body = self.resolve_data(id, body);

                let pattern = match spine {
                    declared::Spine::Single(pattern) => pattern,
                    declared::Spine::Fun { args, .. } => {
                        let span = args
                            .iter()
                            .map(|node| node.span)
                            .reduce(|a, b| a + b)
                            .expect("a function spine has at least one argument");

                        let e = self.errors.parse_error(span).data_parameters_unsupported();
                        let node = resolved::PatternNode::Invalid(e);
                        resolved::Pattern { node, span }
                    }
                };

                resolved::ItemNode::Data(pattern, body)
            }
        };

        resolved::Item { id, node, span }
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
                    let ty = self.ty(item, &mut gen_scope, ty);

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
