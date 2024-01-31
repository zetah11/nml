use super::{Checker, Generic, Scheme};
use crate::frontend::trees::inferred::{MonoPattern, PolyPattern};
use crate::frontend::trees::nodes::PatternNode;

impl<'a, 'src> Checker<'a, '_, 'src, '_> {
    pub(super) fn generalize_pattern(
        &mut self,
        explicit: &[Generic],
        pattern: &MonoPattern<'a>,
    ) -> PolyPattern<'a> {
        let scheme = self.generalize(explicit, pattern.ty);
        self.gen_pattern(&scheme, pattern)
    }

    pub(super) fn monomorphic(&mut self, pattern: &MonoPattern<'a>) -> PolyPattern<'a> {
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

            PatternNode::Constructor(name) => PatternNode::Constructor(*name),
            PatternNode::Group(pattern) => return self.monomorphic(pattern),

            PatternNode::Apply([fun, arg]) => {
                let fun = self.monomorphic(fun);
                let arg = self.monomorphic(arg);
                let terms: &'a [PolyPattern<'a>; 2] = self.alloc.alloc([fun, arg]);
                PatternNode::Apply(terms)
            }

            PatternNode::Or([a, b]) => {
                let a = self.monomorphic(a);
                let b = self.monomorphic(b);
                let terms: &'a [PolyPattern<'a>; 2] = self.alloc.alloc([a, b]);
                PatternNode::Or(terms)
            }

            PatternNode::And([a, b]) => {
                let a = self.monomorphic(a);
                let b = self.monomorphic(b);
                let terms: &'a [PolyPattern<'a>; 2] = self.alloc.alloc([a, b]);
                PatternNode::And(terms)
            }

            PatternNode::Anno(_, v) => match *v {},
        };

        PolyPattern { node, span, scheme }
    }

    fn gen_pattern(&mut self, scheme: &Scheme<'a>, pattern: &MonoPattern<'a>) -> PolyPattern<'a> {
        let span = pattern.span;
        let ty = self.alloc.alloc(self.apply(pattern.ty));
        let scheme = scheme.onto(ty);
        let node = match &pattern.node {
            PatternNode::Invalid(e) => PatternNode::Invalid(*e),
            PatternNode::Wildcard => PatternNode::Wildcard,
            PatternNode::Unit => PatternNode::Unit,

            PatternNode::Bind(name) => {
                self.env.overwrite(*name, scheme.clone());
                PatternNode::Bind(*name)
            }

            PatternNode::Constructor(name) => PatternNode::Constructor(*name),

            PatternNode::Group(pattern) => return self.gen_pattern(&scheme, pattern),

            PatternNode::Apply([fun, arg]) => {
                let fun = self.gen_pattern(&scheme, fun);
                let arg = self.gen_pattern(&scheme, arg);
                let terms: &'a [PolyPattern<'a>; 2] = self.alloc.alloc([fun, arg]);
                PatternNode::Apply(terms)
            }

            PatternNode::Or([a, b]) => {
                let a = self.gen_pattern(&scheme, a);
                let b = self.gen_pattern(&scheme, b);
                let terms: &'a [PolyPattern<'a>; 2] = self.alloc.alloc([a, b]);
                PatternNode::Or(terms)
            }

            PatternNode::And([a, b]) => {
                let a = self.gen_pattern(&scheme, a);
                let b = self.gen_pattern(&scheme, b);
                let terms: &'a [PolyPattern<'a>; 2] = self.alloc.alloc([a, b]);
                PatternNode::And(terms)
            }

            PatternNode::Anno(_, v) => match *v {},
        };

        PolyPattern { node, span, scheme }
    }
}
