use super::Abstractifier;
use crate::parse::cst;
use crate::trees::parsed as ast;

impl<'a, 'lit> Abstractifier<'a, 'lit, '_> {
    pub fn pattern(&mut self, node: &cst::Thing) -> ast::Pattern<'a, 'lit> {
        let span = node.span;
        let node = match &node.node {
            cst::Node::Invalid(e) => ast::PatternNode::Invalid(*e),
            cst::Node::Wildcard => ast::PatternNode::Wildcard,
            cst::Node::Name(_) => return self.name(ast::Affix::Prefix, node),

            cst::Node::Apply(things) => {
                let mut nodes = Vec::with_capacity(things.len());
                let mut affix = None;

                for node in things {
                    if let Some((_, affix)) = affix.take() {
                        nodes.push(self.name(affix, node));
                    } else {
                        match &node.node {
                            cst::Node::Infix => affix = Some((node, ast::Affix::Infix)),
                            cst::Node::Postfix => affix = Some((node, ast::Affix::Postfix)),
                            _ => nodes.push(self.pattern(node)),
                        }
                    }
                }

                if let Some((node, _)) = affix {
                    nodes.push(self.pattern(node));
                }

                let mut nodes = nodes.into_iter();
                let mut fun = nodes.next().expect("`apply` contains at least one node");

                for arg in nodes {
                    let span = fun.span + arg.span;
                    let node = ast::PatternNode::Apply(self.alloc.alloc([fun, arg]));
                    fun = ast::Pattern { node, span };
                }

                return fun;
            }

            _ => ast::PatternNode::Invalid(self.errors.parse_error(span).expected_pattern()),
        };

        ast::Pattern { node, span }
    }

    fn name<'b>(&mut self, affix: ast::Affix, node: &cst::Thing) -> ast::Pattern<'b, 'lit> {
        let span = node.span;
        let node = match &node.node {
            cst::Node::Name(cst::Name::Small(name)) => {
                let name = self.names.intern(name);
                ast::PatternNode::Small((affix, name))
            }

            cst::Node::Name(cst::Name::Operator(name)) => {
                let name = self.names.intern(name);
                ast::PatternNode::Small((affix, name))
            }

            cst::Node::Name(cst::Name::Big(name)) => {
                let name = self.names.intern(name);
                ast::PatternNode::Big((affix, name))
            }

            cst::Node::Invalid(e) => ast::PatternNode::Invalid(*e),

            _ => {
                let e = self.errors.parse_error(span).expected_name();
                ast::PatternNode::Invalid(e)
            }
        };

        ast::Pattern { node, span }
    }
}
