use std::collections::BTreeMap;

use crate::names::{Ident, Name};
use crate::trees::{parsed, resolved};

use super::{ItemId, Resolver};

impl<'a, 'lit> Resolver<'a, 'lit, '_> {
    pub fn expr(
        &mut self,
        item: ItemId,
        gen_scope: &mut BTreeMap<Ident<'lit>, Name>,
        expr: &parsed::Expr<'_, 'lit>,
    ) -> resolved::Expr<'a, 'lit> {
        let span = expr.span;
        let node = match &expr.node {
            parsed::ExprNode::Invalid(e) => resolved::ExprNode::Invalid(*e),

            parsed::ExprNode::Hole => resolved::ExprNode::Hole,
            parsed::ExprNode::Unit => resolved::ExprNode::Unit,

            parsed::ExprNode::Bool(v) => resolved::ExprNode::Bool(*v),

            parsed::ExprNode::Name(name) => {
                if let Some(name) = self.lookup_value(name) {
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
                                let pattern = this.pattern(item, gen_scope, pattern);
                                let body = this.expr(item, gen_scope, body);
                                (pattern, body)
                            })
                        }));

                resolved::ExprNode::Lambda(arrows)
            }

            parsed::ExprNode::Let(binding, [bound, body], ()) => {
                let mut this_scope = BTreeMap::new();

                let binding = self.pattern(item, &mut this_scope, binding);
                let bound = self.expr(item, &mut this_scope, bound);
                self.scope(Self::name_of(&binding), |this| {
                    let body = this.expr(item, &mut this_scope, body);
                    resolved::ExprNode::Let(
                        binding,
                        self.alloc.alloc([bound, body]),
                        self.alloc.alloc_slice_fill_iter(this_scope.into_values()),
                    )
                })
            }

            parsed::ExprNode::Var(v) => match *v {},
        };

        resolved::Expr { node, span }
    }
}
