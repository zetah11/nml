use super::Abstractifier;
use crate::names::Label;
use crate::parse::cst;
use crate::trees::parsed as ast;

impl<'a> Abstractifier<'a, '_> {
    pub fn expr(&mut self, node: &cst::Thing) -> &'a ast::Expr<'a> {
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
                let conditional = self.expr(conditional);
                let consequence = self.expr(consequence);
                let alternative = alternative.map(|node| self.expr(node)).unwrap_or_else(|| {
                    let node = ast::ExprNode::Unit;
                    self.alloc.alloc(ast::Expr { node, span })
                });
                ast::ExprNode::If(conditional, consequence, alternative)
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
                    let node = ast::ExprNode::Field(expr, name, field_span);
                    expr = self.alloc.alloc(ast::Expr { node, span });
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
                    extends.first().map(|node| self.expr(node))
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

            cst::Node::Case { scrutinee, arms } => {
                let scrutinee = self.expr(scrutinee);
                let arms = self.alloc.alloc_slice_fill_with(arms.len(), |idx| {
                    let arm = &arms[idx];
                    self.arrow(arm)
                });

                ast::ExprNode::Case(scrutinee, arms)
            }

            cst::Node::Apply(fun, args) => {
                let mut fun = self.expr(fun);

                for arg in args {
                    let arg = self.expr(arg);
                    let span = fun.span + arg.span;
                    let node = ast::ExprNode::Apply(fun, arg);
                    fun = self.alloc.alloc(ast::Expr { node, span });
                }

                return fun;
            }

            cst::Node::Lambda(pattern, body) => {
                let pattern = self.pattern(pattern);
                let body = self.expr(body);
                ast::ExprNode::Lambda(pattern, body)
            }

            cst::Node::Let { keyword: _, defs, within } => {
                let mut body = if let Some(within) = within {
                    self.expr(within)
                } else {
                    let e = self.errors.parse_error(span).value_definition_without_body();
                    let node = ast::ExprNode::Invalid(e);
                    self.alloc.alloc(ast::Expr { node, span })
                };

                for def in defs.1.iter().rev().chain(std::iter::once(&defs.0)) {
                    let (name, name_span) = self.small_name(def.pattern);
                    let bound = if let Some(bound) = def.definition {
                        self.expr(bound)
                    } else {
                        let span = name_span;
                        let e = self.errors.parse_error(span).missing_definition();
                        let node = ast::ExprNode::Invalid(e);
                        self.alloc.alloc(ast::Expr { node, span })
                    };

                    let span = def.span;
                    let node = ast::ExprNode::Let(name, name_span, bound, body);
                    body = self.alloc.alloc(ast::Expr { node, span });
                }

                return body;
            }
        };

        self.alloc.alloc(ast::Expr { node, span })
    }

    fn arrow(&mut self, node: &cst::Thing) -> (&'a ast::Pattern<'a>, &'a ast::Expr<'a>) {
        match &node.node {
            cst::Node::Invalid(_) => {
                let pattern = self.pattern(node);
                let body = self.expr(node);
                (pattern, body)
            }

            cst::Node::Lambda(pattern, body) => {
                let pattern = self.pattern(pattern);
                let body = self.expr(body);
                (pattern, body)
            }

            _ => {
                let span = node.span;
                let e = self.errors.parse_error(span).expected_case_arm();
                let node = ast::PatternNode::Invalid(e);
                let pattern = self.alloc.alloc(ast::Pattern { node, span });

                let node = ast::ExprNode::Invalid(e);
                let body = self.alloc.alloc(ast::Expr { node, span });

                (pattern, body)
            }
        }
    }
}
