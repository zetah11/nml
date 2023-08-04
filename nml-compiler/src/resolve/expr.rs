use crate::names::Label;
use crate::trees::{declared, resolved};

use super::{ItemId, Resolver};

impl<'a> Resolver<'a, '_> {
    pub fn expr(&mut self, item: ItemId, expr: &declared::Expr) -> &'a resolved::Expr<'a> {
        let span = expr.span;
        let node = match &expr.node {
            declared::ExprNode::Invalid(e) => resolved::ExprNode::Invalid(*e),

            declared::ExprNode::Hole => resolved::ExprNode::Hole,
            declared::ExprNode::Unit => resolved::ExprNode::Unit,

            declared::ExprNode::Small(name) => {
                if let Some(name) = self.lookup_value(name) {
                    resolved::ExprNode::Var(name)
                } else {
                    let name = self.names.get_ident(name);
                    resolved::ExprNode::Invalid(self.errors.name_error(span).unknown_variable(name))
                }
            }

            declared::ExprNode::Big(name) => {
                if self.lookup_value(name).is_some() {
                    todo!("non-anonymous variant")
                } else {
                    resolved::ExprNode::Variant(Label(*name))
                }
            }

            declared::ExprNode::Number(num) => resolved::ExprNode::Number(num.clone()),

            declared::ExprNode::If(cond, then, elze) => {
                let cond = self.expr(item, cond);
                let then = self.expr(item, then);
                let elze = self.expr(item, elze);
                resolved::ExprNode::If(cond, then, elze)
            }

            declared::ExprNode::Field(of, field, field_span) => {
                let of = self.expr(item, of);
                resolved::ExprNode::Field(of, *field, *field_span)
            }

            declared::ExprNode::Record(fields, extend) => {
                let fields = self.alloc.alloc_slice_fill_iter(fields.iter().map(
                    |(label, label_span, def)| {
                        let def = self.expr(item, def);
                        (*label, *label_span, def)
                    },
                ));

                let extend = extend.map(|expr| self.expr(item, expr));

                resolved::ExprNode::Record(fields, extend)
            }

            declared::ExprNode::Case(scrutinee, arms) => {
                let scrutinee = self.expr(item, scrutinee);
                let cases = self.alloc.alloc_slice_fill_iter(arms.iter().map(|(pattern, expr)| {
                    self.scope(None, |this| {
                        let pattern = this.pattern(item, pattern);
                        let expr = this.expr(item, expr);
                        (pattern, expr)
                    })
                }));

                resolved::ExprNode::Case { scrutinee, cases }
            }

            declared::ExprNode::Apply(fun, arg) => {
                let fun = self.expr(item, fun);
                let arg = self.expr(item, arg);
                resolved::ExprNode::Apply(fun, arg)
            }

            declared::ExprNode::Lambda(pattern, body) => self.scope(None, |this| {
                let pattern = this.pattern(item, pattern);
                let body = this.expr(item, body);
                resolved::ExprNode::Lambda(pattern, body)
            }),

            declared::ExprNode::Let(name, name_span, bound, body) => {
                let bound = self.expr(item, bound);
                let name = name.and_then(|name| self.define_value(item, *name_span, name));
                self.scope(name.ok(), |this| {
                    let body = this.expr(item, body);
                    resolved::ExprNode::Let(name, bound, body)
                })
            }
        };

        self.alloc.alloc(resolved::Expr { node, span })
    }
}
