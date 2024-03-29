use std::collections::BTreeMap;

use super::{ItemId, Namespace, Resolver};
use crate::frontend::names::{Ident, Name};
use crate::frontend::trees::declared;
use crate::frontend::trees::{parsed, resolved};

impl<'a, 'scratch, 'src> Resolver<'a, 'scratch, 'src, '_> {
    pub fn expr(
        &mut self,
        item: ItemId,
        gen_scope: &mut BTreeMap<Ident<'src>, Name>,
        expr: &'scratch parsed::Expr<'scratch, 'src>,
    ) -> resolved::Expr<'a, 'src> {
        let span = expr.span;
        let node = match &expr.node {
            parsed::ExprNode::Invalid(e) => resolved::ExprNode::Invalid(*e),

            parsed::ExprNode::Hole => resolved::ExprNode::Hole,
            parsed::ExprNode::Unit => resolved::ExprNode::Unit,

            parsed::ExprNode::Var(name) => {
                if let Some((name, _)) = self.lookup_value(name) {
                    resolved::ExprNode::Var(name)
                } else {
                    let name = name.name();
                    resolved::ExprNode::Invalid(self.errors.name_error(span).unknown_name(name))
                }
            }

            parsed::ExprNode::Number(num) => resolved::ExprNode::Number(num),

            parsed::ExprNode::Anno(expr, ty) => {
                let expr = self.alloc.alloc(self.expr(item, gen_scope, expr));
                let ty = self.resolve_type(item, gen_scope, ty);
                resolved::ExprNode::Anno(expr, ty)
            }

            parsed::ExprNode::Group(expr) => {
                let expr = self.alloc.alloc(self.expr(item, gen_scope, expr));
                resolved::ExprNode::Group(expr)
            }

            parsed::ExprNode::Field(of, field, field_span) => {
                let of = self.expr(item, gen_scope, of);
                let of = self.alloc.alloc(of);
                resolved::ExprNode::Field(of, *field, *field_span)
            }

            parsed::ExprNode::Record(fields, extend) => {
                let fields = self.alloc.alloc_slice_fill_iter(fields.iter().map(
                    |(label, label_span, def)| {
                        let def = self.expr(item, gen_scope, def);
                        (*label, *label_span, def)
                    },
                ));

                let extend =
                    extend.map(|expr| &*self.alloc.alloc(self.expr(item, gen_scope, expr)));

                resolved::ExprNode::Record(fields, extend)
            }

            parsed::ExprNode::Restrict(of, label) => {
                let of = self.expr(item, gen_scope, of);
                let of = self.alloc.alloc(of);
                resolved::ExprNode::Restrict(of, *label)
            }

            parsed::ExprNode::Apply(terms) => return self.apply_expr_run(item, gen_scope, terms),

            parsed::ExprNode::Lambda(arrows) => {
                let arrows =
                    self.alloc
                        .alloc_slice_fill_iter(arrows.iter().map(|(pattern, body)| {
                            self.scope(None, |this| {
                                let pattern = this.single_pattern(item, gen_scope, pattern);
                                let pattern = this.pattern(Namespace::Value, gen_scope, &pattern);
                                let body = this.expr(item, gen_scope, body);
                                (pattern, body)
                            })
                        }));

                resolved::ExprNode::Lambda(arrows)
            }

            parsed::ExprNode::Let(binding, [bound, body], ()) => {
                let mut this_scope = BTreeMap::new();

                let spine = self.function_spine(item, &mut this_scope, binding);
                let (pattern, bound) = match spine {
                    declared::Spine::Single(pattern) => {
                        // Resolve the pattern after the bound body to allow
                        // shadowing
                        let bound = self.expr(item, &mut this_scope, bound);
                        let pattern = self.pattern(Namespace::Value, gen_scope, &pattern);
                        (pattern, bound)
                    }

                    declared::Spine::Fun { head, args, anno } => {
                        // Resolve the pattern before the bound body to allow
                        // local recursive functions
                        let head = self.pattern(Namespace::Value, gen_scope, &head);

                        let bound = if let Some(ty) = anno {
                            let span = bound.span + ty.span;
                            let node = parsed::ExprNode::Anno(bound, *ty);
                            self.scratch.alloc(parsed::Expr { node, span })
                        } else {
                            bound
                        };

                        let body = self.lambda(item, gen_scope, &args, bound);

                        (head, body)
                    }
                };

                self.scope(Self::name_of(&pattern), |this| {
                    let body = this.expr(item, &mut this_scope, body);
                    resolved::ExprNode::Let(
                        pattern,
                        self.alloc.alloc([bound, body]),
                        self.alloc.alloc_slice_fill_iter(this_scope.into_values()),
                    )
                })
            }
        };

        resolved::Expr { node, span }
    }

    pub fn lambda(
        &mut self,
        item_id: ItemId,
        gen_scope: &mut BTreeMap<Ident<'src>, Name>,
        params: &[declared::spined::Pattern<'scratch, 'src>],
        body: &'scratch parsed::Expr<'_, 'src>,
    ) -> resolved::Expr<'a, 'src> {
        if let [param, params @ ..] = params {
            self.scope(None, |this| {
                let pattern = this.pattern(Namespace::Value, gen_scope, param);
                let body = this.lambda(item_id, gen_scope, params, body);
                let span = pattern.span + body.span;
                let node = resolved::ExprNode::Lambda(this.alloc.alloc([(pattern, body)]));
                resolved::Expr { node, span }
            })
        } else {
            self.expr(item_id, gen_scope, body)
        }
    }
}
