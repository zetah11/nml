use crate::names::Label;
use crate::trees::{parsed, resolved};

use super::{ItemId, Resolver};

impl<'a, 'lit> Resolver<'a, 'lit, '_> {
    pub fn expr(
        &mut self,
        item: ItemId,
        expr: &parsed::Expr<'_, 'lit>,
    ) -> resolved::Expr<'a, 'lit> {
        let span = expr.span;
        let node = match &expr.node {
            parsed::ExprNode::Invalid(e) => resolved::ExprNode::Invalid(*e),

            parsed::ExprNode::Hole => resolved::ExprNode::Hole,
            parsed::ExprNode::Unit => resolved::ExprNode::Unit,

            parsed::ExprNode::Bool(v) => resolved::ExprNode::Bool(*v),

            parsed::ExprNode::Small(name) => {
                if let Some(name) = self.lookup_value(name) {
                    resolved::ExprNode::Var(name)
                } else {
                    let name = self.names.get_ident(name);
                    resolved::ExprNode::Invalid(self.errors.name_error(span).unknown_name(name))
                }
            }

            parsed::ExprNode::Big(name) => {
                if self.lookup_value(name).is_some() {
                    todo!("non-anonymous variant")
                } else {
                    resolved::ExprNode::Variant(Label(*name))
                }
            }

            parsed::ExprNode::Number(num) => resolved::ExprNode::Number(num),

            parsed::ExprNode::If([cond, then, elze]) => {
                let cond = self.expr(item, cond);
                let then = self.expr(item, then);
                let elze = self.expr(item, elze);
                resolved::ExprNode::If(self.alloc.alloc([cond, then, elze]))
            }

            parsed::ExprNode::Field(of, field, field_span) => {
                let of = self.expr(item, of);
                let of = self.alloc.alloc(of);
                resolved::ExprNode::Field(of, *field, *field_span)
            }

            parsed::ExprNode::Record(fields, extend) => {
                let fields = self.alloc.alloc_slice_fill_iter(fields.iter().map(
                    |(label, label_span, def)| {
                        let def = self.expr(item, def);
                        (*label, *label_span, def)
                    },
                ));

                let extend = extend.map(|expr| &*self.alloc.alloc(self.expr(item, expr)));

                resolved::ExprNode::Record(fields, extend)
            }

            parsed::ExprNode::Restrict(of, label) => {
                let of = self.expr(item, of);
                let of = self.alloc.alloc(of);
                resolved::ExprNode::Restrict(of, *label)
            }

            parsed::ExprNode::Apply(terms) => return self.apply_run(item, terms),

            parsed::ExprNode::Lambda(arrows) => {
                let arrows =
                    self.alloc
                        .alloc_slice_fill_iter(arrows.iter().map(|(pattern, body)| {
                            self.scope(None, |this| {
                                let pattern = this.pattern(item, pattern);
                                let body = this.expr(item, body);
                                (pattern, body)
                            })
                        }));

                resolved::ExprNode::Lambda(arrows)
            }

            parsed::ExprNode::Let(binding, [bound, body]) => {
                let binding = self.pattern(item, binding);
                let bound = self.expr(item, bound);
                self.scope(Self::name_of(&binding), |this| {
                    let body = this.expr(item, body);
                    resolved::ExprNode::Let(binding, self.alloc.alloc([bound, body]))
                })
            }

            parsed::ExprNode::Variant(v) => match *v {},
            parsed::ExprNode::Var(v) => match *v {},
        };

        resolved::Expr { node, span }
    }
}
