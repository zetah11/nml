use super::Abstractifier;
use crate::parse::cst;
use crate::trees::parsed as ast;

impl<'a, 'lit> Abstractifier<'a, 'lit, '_> {
    pub fn ty(&mut self, node: &cst::Thing) -> ast::Type<'a, 'lit> {
        let span = node.span;
        let node = match &node.node {
            cst::Node::Invalid(e) => ast::TypeNode::Invalid(*e),
            cst::Node::Wildcard => ast::TypeNode::Hole,

            _ => {
                let e = self.errors.parse_error(span).expected_type();
                ast::TypeNode::Invalid(e)
            }
        };

        ast::Type { node, span }
    }
}
