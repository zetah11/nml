use super::Abstractifier;
use crate::names::Label;
use crate::parse::cst;
use crate::trees::parsed as ast;

impl<'a> Abstractifier<'a, '_> {
    pub fn expr(&mut self, node: &cst::Thing) -> ast::Expr<'a> {
        let span = node.span;
        let node = match &node.node {
            cst::Node::Invalid(e) => ast::ExprNode::Invalid(*e),

            cst::Node::Wildcard => ast::ExprNode::Hole,

            cst::Node::Name(cst::Name::Small(name)) => {
                let name = self.names.intern(name);
                ast::ExprNode::Small(name)
            }

            cst::Node::Name(cst::Name::Big(name)) => {
                let name = self.names.intern(name);
                ast::ExprNode::Big(name)
            }

            cst::Node::Number(lit) => {
                let num = Self::parse_number(lit);
                ast::ExprNode::Number(num)
            }

            cst::Node::If { conditional, consequence, alternative } => {
                let cond = self.expr(conditional);
                let cond = self.alloc.alloc(cond);
                let then = self.expr(consequence);
                let then = self.alloc.alloc(then);
                let elze = alternative.map(|node| self.expr(node)).unwrap_or_else(|| {
                    let node = ast::ExprNode::Unit;
                    ast::Expr { node, span }
                });
                let elze = self.alloc.alloc(elze);
                ast::ExprNode::If(cond, then, elze)
            }

            cst::Node::Field(of, fields) => {
                let mut expr = self.expr(of);

                for (field, field_span) in fields {
                    let field_span = *field_span;
                    let name = match field {
                        cst::Name::Small(name) => Ok(self.names.label(name)),
                        cst::Name::Big(name) => {
                            Err(self.errors.parse_error(field_span).expected_name_small(Some(name)))
                        }
                    };

                    let span = expr.span + field_span;
                    let node = ast::ExprNode::Field(self.alloc.alloc(expr), name, field_span);
                    expr = ast::Expr { node, span };
                }

                return expr;
            }

            cst::Node::Record { defs, extends } => {
                let extend = if let Some(span) =
                    extends.iter().skip(1).map(|thing| thing.span).reduce(|a, b| a + b)
                {
                    let e = self.errors.parse_error(span).multiple_record_extensions();
                    let node = ast::ExprNode::Invalid(e);
                    Some(&*self.alloc.alloc(ast::Expr { node, span }))
                } else {
                    extends.first().map(|node| &*self.alloc.alloc(self.expr(node)))
                };

                let fields = self.alloc.alloc_slice_fill_with(defs.len(), |idx| {
                    let def = &defs[idx];
                    let (name, name_span) = self.small_name(def.pattern);
                    let name = name.map(Label);

                    let body = if let Some(body) = def.definition {
                        self.expr(body)
                    } else {
                        self.expr(def.pattern)
                    };

                    (name, name_span, body)
                });

                ast::ExprNode::Record(fields, extend)
            }

            cst::Node::Case(opener, scrutinee, terms) => {
                let cases = self.cases(terms);
                let scrutinee = scrutinee.map(|node| self.expr(node)).unwrap_or_else(|| {
                    let span = *opener;
                    let e = self.errors.parse_error(span).missing_scrutinee();
                    let node = ast::ExprNode::Invalid(e);
                    ast::Expr { node, span }
                });
                let scrutinee = self.alloc.alloc(scrutinee);

                ast::ExprNode::Case { scrutinee, cases }
            }

            cst::Node::Apply(fun, args) => {
                let mut fun = self.expr(fun);

                for arg in args {
                    let arg = self.expr(arg);
                    let arg = self.alloc.alloc(arg);
                    let span = fun.span + arg.span;
                    let node = ast::ExprNode::Apply(self.alloc.alloc(fun), arg);
                    fun = ast::Expr { node, span };
                }

                return fun;
            }

            cst::Node::Arrow(pattern, body) => {
                let pattern = self.pattern(pattern);
                let body = self.expr(body);
                let body = self.alloc.alloc(body);
                ast::ExprNode::Lambda(pattern, body)
            }

            cst::Node::Alt(_) => {
                let e = self.errors.parse_error(span).multiple_lambda_arms();
                ast::ExprNode::Invalid(e)
            }

            cst::Node::Let { keyword: _, defs, within } => {
                let mut body = if let Some(within) = within {
                    self.expr(within)
                } else {
                    let e = self.errors.parse_error(span).value_definition_without_body();
                    let node = ast::ExprNode::Invalid(e);
                    ast::Expr { node, span }
                };

                for def in defs.1.iter().rev().chain(std::iter::once(&defs.0)) {
                    let binding = self.pattern(def.pattern);
                    let bound = if let Some(bound) = def.definition {
                        self.expr(bound)
                    } else {
                        let span = binding.span;
                        let e = self.errors.parse_error(span).missing_definition();
                        let node = ast::ExprNode::Invalid(e);
                        ast::Expr { node, span }
                    };

                    let bound = self.alloc.alloc(bound);

                    let span = def.span;
                    let node = ast::ExprNode::Let(binding, bound, self.alloc.alloc(body));
                    body = ast::Expr { node, span };
                }

                return body;
            }
        };

        ast::Expr { node, span }
    }
}
