use super::Abstractifier;
use crate::names::Ident;
use crate::parse::cst;
use crate::source::Span;
use crate::trees::parsed as ast;

/// Pattern parsing may result in a _function definition spine_ such as `f x y`,
/// which isn't itself a pattern but a name followed by arguments which are
/// individually patterns.
pub enum AbstractPattern<'a, 'lit> {
    Fun(
        (ast::Affix, Ident<'lit>, Span),
        Vec<ast::Pattern<'a, 'lit>>,
        Vec<ast::Type<'a, 'lit>>,
        Span,
    ),
    Single(ast::Pattern<'a, 'lit>),
}

impl AbstractPattern<'_, '_> {
    pub fn span(&self) -> Span {
        match self {
            Self::Fun(.., span) => *span,
            Self::Single(pat) => pat.span,
        }
    }
}

impl<'a, 'lit> Abstractifier<'a, 'lit, '_> {
    pub fn single_pattern(&mut self, node: &cst::Thing) -> ast::Pattern<'a, 'lit> {
        match self.pattern(node) {
            AbstractPattern::Fun(.., span) => {
                let e = self
                    .errors
                    .parse_error(span)
                    .unexpected_function_definition();
                let node = ast::PatternNode::Invalid(e);
                ast::Pattern { node, span }
            }

            AbstractPattern::Single(pattern) => pattern,
        }
    }

    pub fn pattern(&mut self, node: &cst::Thing) -> AbstractPattern<'a, 'lit> {
        let span = node.span;
        let node = match &node.node {
            cst::Node::Invalid(e) => ast::PatternNode::Invalid(*e),
            cst::Node::Wildcard => ast::PatternNode::Wildcard,
            cst::Node::Name(_) => {
                return AbstractPattern::Single(self.name(ast::Affix::Prefix, node))
            }

            cst::Node::Anno(pattern, ty) => {
                let pattern = self.pattern(pattern);
                let ty = self.ty(ty);
                return match pattern {
                    AbstractPattern::Single(pattern) => {
                        let pattern = self.alloc.alloc(pattern);
                        let span = pattern.span + ty.span;
                        let node = ast::PatternNode::Anno(pattern, ty);
                        AbstractPattern::Single(ast::Pattern { node, span })
                    }

                    AbstractPattern::Fun(head, args, mut types, span) => {
                        let span = span + ty.span;
                        types.push(ty);
                        AbstractPattern::Fun(head, args, types, span)
                    }
                };
            }

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
                            _ => nodes.push(self.single_pattern(node)),
                        }
                    }
                }

                if let Some((node, _)) = affix {
                    nodes.push(self.single_pattern(node));
                }

                let mut nodes = nodes.into_iter();
                let mut fun = nodes.next().expect("`apply` contains at least one node");

                if let Some(name) = Self::fun_name(&fun) {
                    let args: Vec<_> = nodes.collect();
                    return AbstractPattern::Fun(name, args, vec![], span);
                } else {
                    for arg in nodes {
                        let span = fun.span + arg.span;
                        let node = ast::PatternNode::Apply(self.alloc.alloc([fun, arg]));
                        fun = ast::Pattern { node, span };
                    }

                    return AbstractPattern::Single(fun);
                }
            }

            _ => ast::PatternNode::Invalid(self.errors.parse_error(span).expected_pattern()),
        };

        let pattern = ast::Pattern { node, span };
        AbstractPattern::Single(pattern)
    }

    fn fun_name(pattern: &ast::Pattern<'a, 'lit>) -> Option<(ast::Affix, Ident<'lit>, Span)> {
        if let ast::PatternNode::Small((affix, name)) = &pattern.node {
            Some((*affix, *name, pattern.span))
        } else {
            None
        }
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
