use crate::trees::inferred::{MonoPattern, PolyPattern};
use crate::trees::nodes::PatternNode;

use super::{Checker, Scheme};

impl<'a> Checker<'a, '_, '_, '_> {
    pub(super) fn generalize<'lit>(
        &mut self,
        pattern: &MonoPattern<'a, 'lit>,
    ) -> PolyPattern<'a, 'lit> {
        let mut pretty = self.pretty.build();
        let scheme = self.solver.generalize(&mut pretty, self.alloc, pattern.ty);
        self.gen_pattern(&scheme, pattern)
    }

    pub(super) fn monomorphic<'lit>(
        &mut self,
        pattern: &MonoPattern<'a, 'lit>,
    ) -> PolyPattern<'a, 'lit> {
        let span = pattern.span;
        let scheme = Scheme::mono(pattern.ty);

        let node = match &pattern.node {
            PatternNode::Invalid(e) => PatternNode::Invalid(*e),
            PatternNode::Wildcard => PatternNode::Wildcard,
            PatternNode::Unit => PatternNode::Unit,

            PatternNode::Bind(name) => {
                self.env.overwrite(*name, scheme.clone());
                PatternNode::Bind(*name)
            }

            PatternNode::Named(name) => PatternNode::Named(*name),

            PatternNode::Deconstruct(label, pattern) => {
                let pattern = self.monomorphic(pattern);
                let pattern = self.alloc.alloc(pattern);
                PatternNode::Deconstruct(*label, pattern)
            }

            PatternNode::Apply(fun, arg) => {
                let fun = self.alloc.alloc(self.monomorphic(fun));
                let arg = self.alloc.alloc(self.monomorphic(arg));
                PatternNode::Apply(fun, arg)
            }

            PatternNode::Small(v) | PatternNode::Big(v) => match *v {},
        };

        PolyPattern { node, span, scheme }
    }

    fn gen_pattern<'lit>(
        &mut self,
        scheme: &Scheme<'a>,
        pattern: &MonoPattern<'a, 'lit>,
    ) -> PolyPattern<'a, 'lit> {
        let span = pattern.span;
        let ty = self.solver.apply(self.alloc, pattern.ty);
        let scheme = scheme.onto(ty);
        let node = match &pattern.node {
            PatternNode::Invalid(e) => PatternNode::Invalid(*e),
            PatternNode::Wildcard => PatternNode::Wildcard,
            PatternNode::Unit => PatternNode::Unit,

            PatternNode::Bind(name) => {
                self.env.overwrite(*name, scheme.clone());
                PatternNode::Bind(*name)
            }

            PatternNode::Named(name) => PatternNode::Named(*name),

            PatternNode::Deconstruct(label, pattern) => {
                let pattern = self.gen_pattern(&scheme, pattern);
                let pattern = self.alloc.alloc(pattern);
                PatternNode::Deconstruct(*label, pattern)
            }

            PatternNode::Apply(fun, arg) => {
                let fun = self.alloc.alloc(self.gen_pattern(&scheme, fun));
                let arg = self.alloc.alloc(self.gen_pattern(&scheme, arg));
                PatternNode::Apply(fun, arg)
            }

            PatternNode::Small(v) | PatternNode::Big(v) => match *v {},
        };

        PolyPattern { node, span, scheme }
    }
}
