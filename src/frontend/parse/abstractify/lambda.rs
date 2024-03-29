use super::Abstractifier;
use crate::frontend::parse::cst::{self};
use crate::frontend::trees::parsed as ast;

impl<'a, 'src> Abstractifier<'a, 'src, '_> {
    pub(super) fn cases(
        &mut self,
        node: &cst::Thing<'_, 'src>,
    ) -> &'a [(ast::Pattern<'a, 'src>, ast::Expr<'a, 'src>)] {
        if let cst::Node::Alt(lambdas) = &node.node {
            self.alloc
                .alloc_slice_fill_iter(lambdas.iter().map(|node| self.arrow(node)))
        } else {
            self.alloc
                .alloc_slice_fill_iter(std::iter::once(self.arrow(node)))
        }
    }

    pub(super) fn arrow(
        &mut self,
        node: &cst::Thing<'_, 'src>,
    ) -> (ast::Pattern<'a, 'src>, ast::Expr<'a, 'src>) {
        let span = node.span;
        match &node.node {
            cst::Node::Invalid(e) => {
                let node = ast::PatternNode::Invalid(*e);
                let expr = ast::ExprNode::Invalid(*e);
                let pattern = ast::Pattern { node, span };
                let expr = ast::Expr { node: expr, span };
                (pattern, expr)
            }

            cst::Node::Arrow(pattern, body) => {
                let pattern = self.pattern(pattern);
                let body = self.expr(body);
                (pattern, body)
            }

            cst::Node::Group(node) => {
                return self.arrow(node);
            }

            _ => {
                let expr = self.expr(node);
                let e = self.errors.parse_error(span).expected_case_arm();
                let node = ast::PatternNode::Invalid(e);
                (ast::Pattern { node, span }, expr)
            }
        }
    }
}
