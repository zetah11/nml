use super::Abstractifier;
use crate::frontend::errors::ErrorId;
use crate::frontend::names::Label;
use crate::frontend::parse::cst;
use crate::frontend::source::Span;
use crate::frontend::trees::parsed as ast;

impl<'a, 'lit> Abstractifier<'a, 'lit, '_> {
    pub fn expr(&mut self, node: &cst::Thing) -> ast::Expr<'a, 'lit> {
        let span = node.span;
        let node = match &node.node {
            cst::Node::Invalid(e) => ast::ExprNode::Invalid(*e),

            cst::Node::Wildcard => ast::ExprNode::Hole,

            cst::Node::Name(cst::Name::Normal(name)) => {
                let name = self.names.intern(name);
                ast::ExprNode::Var(name)
            }

            cst::Node::Name(cst::Name::Universal(name)) => {
                let e = self
                    .errors
                    .parse_error(span)
                    .expected_non_universal_name(name);
                ast::ExprNode::Invalid(e)
            }

            cst::Node::Number(lit) => {
                let num = self.parse_number(lit);
                ast::ExprNode::Number(num)
            }

            cst::Node::Anno(expr, ty) => {
                let expr = self.alloc.alloc(self.expr(expr));
                let ty = self.ty(ty);
                ast::ExprNode::Anno(expr, ty)
            }

            cst::Node::Group(expr) => {
                let expr = self.alloc.alloc(self.expr(expr));
                ast::ExprNode::Group(expr)
            }

            cst::Node::Field(of, fields) => {
                let mut expr = self.expr(of);

                for (field, field_span) in fields {
                    let field_span = *field_span;
                    let name = match field {
                        cst::Name::Normal(name) => Ok(self.names.label(name)),

                        cst::Name::Universal(name) => Err(self
                            .errors
                            .parse_error(field_span)
                            .expected_non_universal_name(name)),
                    };

                    let span = expr.span + field_span;
                    let node = ast::ExprNode::Field(self.alloc.alloc(expr), name, field_span);
                    expr = ast::Expr { node, span };
                }

                return expr;
            }

            cst::Node::Record { defs } => self.record(defs),

            cst::Node::Case(scrutinee, terms) => {
                let span = terms.span;
                let cases = self.cases(terms);
                let node = ast::ExprNode::Lambda(cases);

                if let Some(scrutinee) = scrutinee {
                    let case = ast::Expr { node, span };
                    let scrutinee = self.expr(scrutinee);

                    let exprs = self.alloc.alloc([case, scrutinee]);
                    ast::ExprNode::Apply(exprs)
                } else {
                    node
                }
            }

            cst::Node::Apply(things) => {
                let exprs = self
                    .alloc
                    .alloc_slice_fill_iter(things.iter().map(|node| self.expr(node)));
                ast::ExprNode::Apply(exprs)
            }

            cst::Node::Arrow(pattern, body) => {
                let pattern = self.pattern(pattern);
                let body = self.expr(body);
                ast::ExprNode::Lambda(
                    self.alloc
                        .alloc_slice_fill_iter(std::iter::once((pattern, body))),
                )
            }

            cst::Node::Alt(nodes) => {
                let arrows = self
                    .alloc
                    .alloc_slice_fill_iter(nodes.iter().map(|node| self.arrow(node)));
                ast::ExprNode::Lambda(arrows)
            }

            cst::Node::Let {
                kw: (cst::LetKw::Let, _),
                defs,
                within,
            } => {
                let mut body = if let Some(within) = within {
                    self.expr(within)
                } else {
                    let e = self
                        .errors
                        .parse_error(span)
                        .value_definition_without_body();
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

                    let span = def.span;
                    let node = ast::ExprNode::Let(binding, self.alloc.alloc([bound, body]), ());
                    body = ast::Expr { node, span };
                }

                return body;
            }

            _ => {
                let e = self.errors.parse_error(span).expected_expr();
                ast::ExprNode::Invalid(e)
            }
        };

        ast::Expr { node, span }
    }

    fn record(&mut self, defs: &[cst::ValueDef]) -> ast::ExprNode<'a, 'lit> {
        let mut extend = None;

        let fields: Vec<_> = defs
            .iter()
            .flat_map(|def| self.record_field(def, &mut extend))
            .collect();

        let fields = self.alloc.alloc_slice_fill_iter(fields);

        let extend = match extend {
            Some(Ok(term)) => Some(term),
            Some(Err(span)) => {
                let e = self.errors.parse_error(span).multiple_record_extensions();
                let node = ast::ExprNode::Invalid(e);
                Some(ast::Expr { node, span })
            }
            None => None,
        };

        let extend = extend.map(|expr| &*self.alloc.alloc(expr));
        ast::ExprNode::Record(fields, extend)
    }

    /// Parse a single record field or record extension. Returns
    /// `Some((name, name_span, body))` for a record field definition like
    /// `a = x` or `a` (where the latter is expanded to `a = a`). For record
    /// extensions, returns `None` and writes to `extend`, which is either the
    /// (optional) single record extension, or the span containing all multiple
    /// record extensions met so far.
    fn record_field(
        &mut self,
        def: &cst::ValueDef<'_>,
        extend: &mut Option<Result<ast::Expr<'a, 'lit>, Span>>,
    ) -> Option<(Result<Label<'lit>, ErrorId>, Span, ast::Expr<'a, 'lit>)> {
        if let Some(extension_terms) = Self::get_record_extension(def.pattern) {
            // Get the extension term
            let mut term = match extension_terms {
                [] => unreachable!("record extensions are ellipses applied to at least one thing"),
                [term] => self.expr(term),
                terms @ [first, .., last] => {
                    let span = first.span + last.span;
                    let node = cst::Node::Apply(terms.to_vec());
                    let term = cst::Thing { node, span };
                    self.expr(&term)
                }
            };

            // Error on `... x = y`
            if let Some(definition) = def.definition {
                let span = definition.span;
                let e = self
                    .errors
                    .parse_error(span)
                    .record_extension_with_definition();
                let node = ast::ExprNode::Invalid(e);
                term = ast::Expr { node, span };
            }

            // Return the extension
            *extend = match extend.take() {
                Some(Ok(previous)) => Some(Err(previous.span + term.span)),
                Some(Err(span)) => Some(Err(span + term.span)),
                None => Some(Ok(term)),
            };

            None
        } else {
            let (name, name_span) = self.normal_name(def.pattern);
            let name = name.map(Label);

            let body = if let Some(body) = def.definition {
                self.expr(body)
            } else {
                self.expr(def.pattern)
            };

            Some((name, name_span, body))
        }
    }

    /// Returns `Some(x y z) if the given node is an application like
    /// `... x y z`.
    fn get_record_extension<'tree>(
        node: &'tree cst::Thing<'tree>,
    ) -> Option<&'tree [&'tree cst::Thing<'tree>]> {
        let cst::Node::Apply(terms) = &node.node else {
            return None;
        };
        match &terms[..] {
            [cst::Thing {
                node: cst::Node::Ellipses,
                ..
            }, rest @ ..] => Some(rest),
            _ => None,
        }
    }
}
