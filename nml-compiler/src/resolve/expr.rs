use std::collections::BTreeMap;

use super::{ItemId, Resolver};
use crate::names::{Ident, Name};
use crate::trees::declared;
use crate::trees::{parsed, resolved};

impl<'a, 'scratch, 'lit> Resolver<'a, 'scratch, 'lit, '_> {
    pub fn expr(
        &mut self,
        item: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        expr: &'scratch parsed::Expr<'scratch, 'lit>,
    ) -> resolved::Expr<'a, 'lit> {
        let span = expr.span;
        let node = match &expr.node {
            parsed::ExprNode::Invalid(e) => resolved::ExprNode::Invalid(*e),

            parsed::ExprNode::Hole => resolved::ExprNode::Hole,
            parsed::ExprNode::Unit => resolved::ExprNode::Unit,

            parsed::ExprNode::Bool(v) => resolved::ExprNode::Bool(*v),

            parsed::ExprNode::Name(name) => {
                if let Some((name, _)) = self.lookup_value(name) {
                    resolved::ExprNode::Var(name)
                } else {
                    let name = self.names.get_ident(name);
                    resolved::ExprNode::Invalid(self.errors.name_error(span).unknown_name(name))
                }
            }

            parsed::ExprNode::Number(num) => resolved::ExprNode::Number(num),

            parsed::ExprNode::If([cond, then, elze]) => {
                let cond = self.expr(item, gen_scope, cond);
                let then = self.expr(item, gen_scope, then);
                let elze = self.expr(item, gen_scope, elze);
                resolved::ExprNode::If(self.alloc.alloc([cond, then, elze]))
            }

            parsed::ExprNode::Anno(expr, ty) => {
                let expr = self.alloc.alloc(self.expr(item, gen_scope, expr));
                let ty = self.ty(item, gen_scope, ty);
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
                                let pattern = this.pattern(gen_scope, &pattern);
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
                        let pattern = self.pattern(gen_scope, &pattern);
                        (pattern, bound)
                    }

                    declared::Spine::Fun { head, args } => {
                        // Resolve the pattern before the bound body to allow
                        // local recursive functions
                        let head = self.pattern(gen_scope, &head);
                        let body = self.lambda(
                            item,
                            gen_scope,
                            &args,
                            bound,
                            |this, gen_scope, pattern| this.pattern(gen_scope, pattern),
                        );
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

            parsed::ExprNode::Var(v) => match *v {},
        };

        resolved::Expr { node, span }
    }

    pub fn lambda<T>(
        &mut self,
        item_id: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        params: &[T],
        body: &'scratch parsed::Expr<'_, 'lit>,
        mut f: impl FnMut(
            &mut Self,
            &mut BTreeMap<Ident<'lit>, Name>,
            &T,
        ) -> resolved::Pattern<'a, 'lit>,
    ) -> resolved::Expr<'a, 'lit> {
        if let [param, params @ ..] = params {
            self.scope(None, |this| {
                let pattern = f(this, gen_scope, param);
                let body = this.lambda(item_id, gen_scope, params, body, f);
                let span = pattern.span + body.span;
                let node = resolved::ExprNode::Lambda(this.alloc.alloc([(pattern, body)]));
                resolved::Expr { node, span }
            })
        } else {
            self.expr(item_id, gen_scope, body)
        }
    }
}
