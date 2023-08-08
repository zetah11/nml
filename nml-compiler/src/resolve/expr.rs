use crate::names::Label;
use crate::trees::{declared, resolved};

use super::{ItemId, Resolver};

impl<'a> Resolver<'a, '_> {
    pub fn expr(&mut self, item: ItemId, expr: &declared::Expr) -> resolved::Expr<'a> {
        let span = expr.span;
        let node = match &expr.node {
            declared::ExprNode::Invalid(e) => resolved::ExprNode::Invalid(*e),

            declared::ExprNode::Hole => resolved::ExprNode::Hole,
            declared::ExprNode::Unit => resolved::ExprNode::Unit,

            declared::ExprNode::Bool(v) => resolved::ExprNode::Bool(*v),

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
                let cond = self.alloc.alloc(cond);
                let then = self.expr(item, then);
                let then = self.alloc.alloc(then);
                let elze = self.expr(item, elze);
                let elze = self.alloc.alloc(elze);
                resolved::ExprNode::If(cond, then, elze)
            }

            declared::ExprNode::Field(of, field, field_span) => {
                let of = self.expr(item, of);
                let of = self.alloc.alloc(of);
                resolved::ExprNode::Field(of, *field, *field_span)
            }

            declared::ExprNode::Record(fields, extend) => {
                let fields = self.alloc.alloc_slice_fill_iter(fields.iter().map(
                    |(label, label_span, def)| {
                        let def = self.expr(item, def);
                        (*label, *label_span, def)
                    },
                ));

                let extend = extend.map(|expr| &*self.alloc.alloc(self.expr(item, expr)));

                resolved::ExprNode::Record(fields, extend)
            }

            declared::ExprNode::Restrict(of, label) => {
                let of = self.expr(item, of);
                let of = self.alloc.alloc(of);
                resolved::ExprNode::Restrict(of, *label)
            }

            declared::ExprNode::Case { scrutinee, cases } => {
                let scrutinee = self.expr(item, scrutinee);
                let scrutinee = self.alloc.alloc(scrutinee);

                let cases =
                    self.alloc.alloc_slice_fill_iter(cases.iter().map(|(pattern, expr)| {
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
                let fun = self.alloc.alloc(fun);
                let arg = self.expr(item, arg);
                let arg = self.alloc.alloc(arg);
                resolved::ExprNode::Apply(fun, arg)
            }

            declared::ExprNode::Lambda(pattern, body) => self.scope(None, |this| {
                let pattern = this.pattern(item, pattern);
                let body = this.expr(item, body);
                let body = self.alloc.alloc(body);
                resolved::ExprNode::Lambda(pattern, body)
            }),

            declared::ExprNode::Let(binding, bound, body) => {
                let binding = self.pattern(item, binding);
                let bound = self.expr(item, bound);
                let bound = self.alloc.alloc(bound);
                self.scope(Self::name_of(&binding), |this| {
                    let body = this.expr(item, body);
                    let body = self.alloc.alloc(body);
                    resolved::ExprNode::Let(binding, bound, body)
                })
            }

            declared::ExprNode::Variant(v) => match *v {},
            declared::ExprNode::Var(v) => match *v {},
        };

        resolved::Expr { node, span }
    }
}
