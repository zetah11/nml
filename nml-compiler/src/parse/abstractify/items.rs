use bumpalo::collections::Vec;

use super::pattern::AbstractPattern;
use super::Abstractifier;
use crate::parse::cst;
use crate::trees::parsed as ast;

impl<'a, 'lit> Abstractifier<'a, 'lit, '_> {
    pub fn item(&mut self, into: &mut Vec<ast::Item<'a, 'lit>>, node: &cst::Thing) {
        let span = node.span;
        let node = match &node.node {
            cst::Node::Invalid(e) => ast::ItemNode::Invalid(*e),
            cst::Node::Let { defs, within } => {
                if let Some(within) = within {
                    let e = self
                        .errors
                        .parse_error(within.span)
                        .item_definition_with_body();
                    self.parse_errors.push((e, within.span));
                }

                into.reserve_exact(defs.1.len() + 1);
                into.push(self.single_value(&defs.0));
                into.extend(defs.1.iter().map(|def| self.single_value(def)));
                return;
            }

            _ => {
                let e = self.errors.parse_error(span).expected_item();
                ast::ItemNode::Invalid(e)
            }
        };

        into.push(ast::Item { node, span });
    }

    fn single_value(&mut self, def: &cst::ValueDef) -> ast::Item<'a, 'lit> {
        let pattern = self.pattern(def.pattern);

        let body = def
            .definition
            .map(|node| self.expr(node))
            .unwrap_or_else(|| {
                let span = pattern.span();
                let e = self.errors.parse_error(span).missing_definition();
                let node = ast::ExprNode::Invalid(e);
                ast::Expr { node, span }
            });

        let (pattern, body) = match pattern {
            AbstractPattern::Fun((affix, name, name_span), args, _) => {
                let node = ast::PatternNode::Small((affix, name));
                let pattern = ast::Pattern {
                    node,
                    span: name_span,
                };

                let mut body = body;
                for arg in args.into_iter().rev() {
                    let span = arg.span + body.span;
                    let terms = self.alloc.alloc([(arg, body)]);
                    let node = ast::ExprNode::Lambda(terms);
                    body = ast::Expr { node, span };
                }

                (pattern, body)
            }

            AbstractPattern::Single(pattern) => (pattern, body),
        };

        let span = def.span;
        let node = ast::ItemNode::Let(pattern, body);
        ast::Item { node, span }
    }
}
