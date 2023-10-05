use super::Abstractifier;
use crate::messages::parse::NonSmallName;
use crate::parse::cst;
use crate::trees::parsed as ast;

impl<'a, 'lit> Abstractifier<'a, 'lit, '_> {
    /// Abstract an applied sequence of patterns. A resulting slice with a
    /// length longer than 1 does _not_ mean that this is a function definition.
    /// Any type annotations "outside" the spine are collected and returned
    /// separately.
    pub fn pattern(&mut self, node: &cst::Thing) -> ast::Pattern<'a, 'lit> {
        let span = node.span;
        let node = match &node.node {
            cst::Node::Invalid(e) => ast::PatternNode::Invalid(*e),
            cst::Node::Wildcard => ast::PatternNode::Wildcard,

            cst::Node::Name(_) => return self.affixed_name(ast::Affix::Prefix, node),

            cst::Node::Anno(pat, ty) => {
                let pat = self.alloc.alloc(self.pattern(pat));
                let ty = self.ty(ty);
                ast::PatternNode::Anno(pat, ty)
            }

            cst::Node::Apply(terms) => {
                let mut nodes = Vec::with_capacity(terms.len());
                let mut affix = None;

                for node in terms {
                    if let Some((_, affix)) = affix.take() {
                        nodes.push(self.affixed_name(affix, node));
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

                if nodes.len() == 1 {
                    return nodes.remove(0);
                }

                let terms = self.alloc.alloc_slice_fill_iter(nodes);
                ast::PatternNode::Apply(terms)
            }

            _ => {
                let e = self.errors.parse_error(span).expected_pattern();
                ast::PatternNode::Invalid(e)
            }
        };

        ast::Pattern { node, span }
    }

    fn affixed_name(
        &mut self,
        affix: ast::Affix,
        suspected_name: &cst::Thing,
    ) -> ast::Pattern<'a, 'lit> {
        let span = suspected_name.span;
        let node = match &suspected_name.node {
            cst::Node::Invalid(e) => ast::PatternNode::Invalid(*e),

            cst::Node::Name(cst::Name::Normal(name)) => {
                let name = self.names.intern(name);
                ast::PatternNode::Bind((affix, name))
            }

            cst::Node::Name(cst::Name::Universal(name)) => {
                let e = self
                    .errors
                    .parse_error(span)
                    .expected_name_small(NonSmallName::Universal(name));
                ast::PatternNode::Invalid(e)
            }

            cst::Node::Group(inner) => {
                let pattern = self.alloc.alloc(self.affixed_name(affix, inner));
                ast::PatternNode::Group(pattern)
            }

            _ => {
                let e = self.errors.parse_error(span).expected_name();
                ast::PatternNode::Invalid(e)
            }
        };

        ast::Pattern { node, span }
    }
}
