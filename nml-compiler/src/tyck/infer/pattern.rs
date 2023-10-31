use super::{Checker, Scheme, Type};
use crate::trees::{inferred as o, resolved as i};

impl<'a, 'lit> Checker<'a, '_, 'lit, '_> {
    pub fn infer_pattern(
        &mut self,
        wildcards: &mut Vec<&'a Type<'a>>,
        pattern: &i::Pattern<'_, 'lit>,
    ) -> o::MonoPattern<'a> {
        let span = pattern.span;
        let (node, ty) = match &pattern.node {
            i::PatternNode::Invalid(e) => (
                o::MonoPatternNode::Invalid(*e),
                &*self.alloc.alloc(Type::Invalid(*e)),
            ),

            i::PatternNode::Wildcard => {
                (o::MonoPatternNode::Wildcard, self.wildcard_type(wildcards))
            }

            i::PatternNode::Unit => (o::MonoPatternNode::Unit, &*self.alloc.alloc(Type::Unit)),

            i::PatternNode::Bind(name) => {
                let ty = self.wildcard_type(wildcards);
                self.env.insert(*name, Scheme::mono(ty));
                (o::MonoPatternNode::Bind(*name), ty)
            }

            i::PatternNode::Constructor(name) => {
                let scheme = self.env.lookup(name);
                let mut pretty = self.pretty.build();
                let ty = self.solver.instantiate(&mut pretty, self.alloc, scheme);
                let ty = &*self.alloc.alloc(ty);
                (o::MonoPatternNode::Constructor(*name), ty)
            }

            i::PatternNode::Anno(pattern, ty) => {
                let pattern = self.infer_pattern(wildcards, pattern);
                let ty = self.lower(ty);

                let mut pretty = self.pretty.build();
                self.solver
                    .unify(&mut pretty, self.alloc, self.errors, span, ty, pattern.ty);

                return pattern;
            }

            i::PatternNode::Group(pattern) => return self.infer_pattern(wildcards, pattern),

            i::PatternNode::Apply([ctr, arg]) => {
                let ctr = self.infer_pattern(wildcards, ctr);
                let arg = self.infer_pattern(wildcards, arg);

                let res_ty = self.fresh();
                let fun_ty = self.alloc.alloc(Type::Fun(arg.ty, res_ty));

                let mut pretty = self.pretty.build();
                self.solver
                    .unify(&mut pretty, self.alloc, self.errors, span, ctr.ty, fun_ty);

                let terms = self.alloc.alloc([ctr, arg]);
                (o::MonoPatternNode::Apply(terms), res_ty)
            }
        };

        o::MonoPattern { node, span, ty }
    }

    fn wildcard_type(&mut self, wildcards: &mut Vec<&'a Type<'a>>) -> &'a Type<'a> {
        let ty = self.fresh();
        wildcards.push(ty);
        ty
    }
}
