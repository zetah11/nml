use super::Abstractifier;
use crate::parse::cst;
use crate::trees::parsed as ast;

impl<'a> Abstractifier<'a, '_> {
    pub fn pattern(&mut self, node: &cst::Thing) -> ast::Pattern<'a> {
        let span = node.span;
        let node = match &node.node {
            cst::Node::Invalid(e) => ast::PatternNode::Invalid(*e),

            cst::Node::Wildcard => ast::PatternNode::Wildcard,

            cst::Node::Name(cst::Name::Small(name)) => {
                let name = self.names.intern(name);
                ast::PatternNode::Small(name)
            }

            cst::Node::Name(cst::Name::Big(name)) => {
                let name = self.names.intern(name);
                ast::PatternNode::Big(name)
            }

            cst::Node::Apply(fun, args) => {
                let mut fun = self.pattern(fun);

                for arg in args {
                    let arg = self.pattern(arg);
                    let arg = self.alloc.alloc(arg);
                    let span = fun.span + arg.span;
                    let node = ast::PatternNode::Apply(self.alloc.alloc(fun), arg);
                    fun = ast::Pattern { node, span };
                }

                return fun;
            }

            _ => ast::PatternNode::Invalid(self.errors.parse_error(span).expected_pattern()),
        };

        ast::Pattern { node, span }
    }
}
