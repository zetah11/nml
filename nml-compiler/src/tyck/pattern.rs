use crate::trees::inferred::{MonoPattern, PolyPattern};
use crate::trees::nodes::PatternNode;

use super::{Checker, Generic, Scheme};

impl<'a, 'ids> Checker<'a, '_, 'ids, '_> {
    pub(super) fn generalize(
        &mut self,
        explicit: &[Generic],
        pattern: &MonoPattern<'a, 'ids>,
    ) -> PolyPattern<'a, 'ids> {
        let mut pretty = self.pretty.build();
        let scheme = self
            .solver
            .generalize(&mut pretty, self.alloc, explicit, pattern.ty);

        self.gen_pattern(&scheme, pattern)
    }

    pub(super) fn monomorphic(&mut self, pattern: &MonoPattern<'a, 'ids>) -> PolyPattern<'a, 'ids> {
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

            PatternNode::Apply([fun, arg]) => {
                let fun = self.monomorphic(fun);
                let arg = self.monomorphic(arg);
                PatternNode::Apply(self.alloc.alloc([fun, arg]))
            }

            PatternNode::Name(v) | PatternNode::Anno(_, v) => match *v {},
        };

        PolyPattern { node, span, scheme }
    }

    fn gen_pattern(
        &mut self,
        scheme: &Scheme<'a>,
        pattern: &MonoPattern<'a, 'ids>,
    ) -> PolyPattern<'a, 'ids> {
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

            PatternNode::Apply([fun, arg]) => {
                let fun = self.gen_pattern(&scheme, fun);
                let arg = self.gen_pattern(&scheme, arg);
                PatternNode::Apply(self.alloc.alloc([fun, arg]))
            }

            PatternNode::Name(v) | PatternNode::Anno(_, v) => match *v {},
        };

        PolyPattern { node, span, scheme }
    }
}
