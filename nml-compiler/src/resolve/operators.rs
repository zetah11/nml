use super::{ItemId, Resolver};
use crate::names::Name;
use crate::trees::parsed::Affix;
use crate::trees::{parsed, resolved};

impl<'a, 'lit> Resolver<'a, 'lit, '_> {
    pub(super) fn apply_run(
        &mut self,
        item: ItemId,
        terms: &[parsed::Expr<'_, 'lit>],
    ) -> resolved::Expr<'a, 'lit> {
        let mut infix: Option<(Vec<resolved::Expr>, resolved::Expr, Name)> = None;
        let mut exprs: Vec<resolved::Expr> = Vec::with_capacity(terms.len());

        for term in terms {
            let term = self.expr(item, term);

            match &term.node {
                resolved::ExprNode::Var(name) if self.affii.get(name) == Some(&Affix::Postfix) => {
                    if let Some(prev) = exprs.pop() {
                        let span = term.span + prev.span;
                        let node = resolved::ExprNode::Apply(self.alloc.alloc([term, prev]));
                        exprs.push(resolved::Expr { node, span });
                    } else {
                        let span = term.span;
                        let name = self.names.get_name(name);
                        let name = self.names.get_ident(&name.name);
                        let e = self.errors.parse_error(span).postfix_function(name);
                        let node = resolved::ExprNode::Invalid(e);
                        exprs.push(resolved::Expr { node, span });
                    }
                }

                resolved::ExprNode::Var(name) if self.affii.get(name) == Some(&Affix::Infix) => {
                    if let Some((_, op, _)) = infix.as_ref() {
                        let span = term.span;
                        let e = self
                            .errors
                            .parse_error(span)
                            .ambiguous_infix_operators(op.span);
                        let node = resolved::ExprNode::Invalid(e);
                        exprs.push(resolved::Expr { node, span });
                    } else if exprs.is_empty() {
                        let span = term.span;
                        let name = self.names.get_name(name);
                        let name = self.names.get_ident(&name.name);
                        let e = self.errors.parse_error(span).infix_function(name);
                        let node = resolved::ExprNode::Invalid(e);
                        exprs.push(resolved::Expr { node, span });
                    } else {
                        let lhs = std::mem::take(&mut exprs);
                        let name = *name;
                        infix = Some((lhs, term, name));
                    }
                }

                _ => exprs.push(term),
            }
        }

        if let Some((mut lhs, op, name)) = infix {
            let lhs = {
                let fun = lhs.remove(0);
                self.prefixes(fun, lhs)
            };

            let rhs = if exprs.is_empty() {
                let span = op.span;
                let name = self.names.get_name(&name);
                let name = self.names.get_ident(&name.name);
                let e = self.errors.parse_error(span).infix_function(name);
                let node = resolved::ExprNode::Invalid(e);
                resolved::Expr { node, span }
            } else {
                let fun = exprs.remove(0);
                self.prefixes(fun, exprs)
            };

            let span = lhs.span + op.span;
            let node = resolved::ExprNode::Apply(self.alloc.alloc([op, lhs]));
            let fun = resolved::Expr { node, span };

            let span = fun.span + rhs.span;
            let node = resolved::ExprNode::Apply(self.alloc.alloc([fun, rhs]));
            resolved::Expr { node, span }
        } else {
            let fun = exprs.remove(0);
            self.prefixes(fun, exprs)
        }
    }

    fn prefixes(
        &self,
        mut fun: resolved::Expr<'a, 'lit>,
        args: Vec<resolved::Expr<'a, 'lit>>,
    ) -> resolved::Expr<'a, 'lit> {
        for arg in args {
            let span = fun.span + arg.span;
            let node = resolved::ExprNode::Apply(self.alloc.alloc([fun, arg]));
            fun = resolved::Expr { node, span };
        }

        fun
    }
}
