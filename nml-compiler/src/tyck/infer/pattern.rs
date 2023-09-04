use super::{Checker, Row, Scheme, Type};
use crate::trees::{inferred as o, resolved as i};

impl<'a> Checker<'a, '_, '_, '_> {
    pub fn infer_pattern<'lit>(
        &mut self,
        wildcards: &mut Vec<&'a Type<'a>>,
        pattern: &i::Pattern<'_, 'lit>,
    ) -> o::MonoPattern<'a, 'lit> {
        let span = pattern.span;
        let (node, ty) = match &pattern.node {
            i::PatternNode::Invalid(e) => {
                (o::MonoPatternNode::Invalid(*e), &*self.alloc.alloc(Type::Invalid(*e)))
            }

            i::PatternNode::Wildcard => {
                (o::MonoPatternNode::Wildcard, self.wildcard_type(wildcards))
            }

            i::PatternNode::Unit => (o::MonoPatternNode::Unit, &*self.alloc.alloc(Type::Unit)),

            i::PatternNode::Bind(name) => {
                let ty = self.wildcard_type(wildcards);
                self.env.insert(*name, Scheme::mono(ty));
                (o::MonoPatternNode::Bind(*name), ty)
            }

            i::PatternNode::Named(name) => {
                let scheme = self.env.lookup(name);
                let mut pretty = self.pretty.build();
                let ty = self.solver.instantiate(&mut pretty, self.alloc, scheme);
                (o::MonoPatternNode::Named(*name), ty)
            }

            i::PatternNode::Deconstruct(label, pattern) => {
                let pattern = self.infer_pattern(wildcards, pattern);
                let pattern = self.alloc.alloc(pattern);
                let row_ty = self.fresh_row();
                let row_ty = self.alloc.alloc(Row::Extend(*label, pattern.ty, row_ty));
                let ty = &*self.alloc.alloc(Type::Variant(row_ty));
                (o::MonoPatternNode::Deconstruct(*label, pattern), ty)
            }

            i::PatternNode::Apply(ctr, arg) => {
                let ctr = self.infer_pattern(wildcards, ctr);
                let ctr = self.alloc.alloc(ctr);
                let arg = self.infer_pattern(wildcards, arg);
                let arg = self.alloc.alloc(arg);

                let res_ty = self.fresh();
                let fun_ty = self.alloc.alloc(Type::Fun(arg.ty, res_ty));

                let mut pretty = self.pretty.build();
                self.solver.unify(&mut pretty, self.alloc, self.errors, span, ctr.ty, fun_ty);

                (o::MonoPatternNode::Apply(ctr, arg), res_ty)
            }

            i::PatternNode::Small(v) | i::PatternNode::Big(v) => match *v {},
        };

        o::MonoPattern { node, span, ty }
    }

    fn wildcard_type(&mut self, wildcards: &mut Vec<&'a Type<'a>>) -> &'a Type<'a> {
        let ty = self.fresh();
        wildcards.push(ty);
        ty
    }
}
