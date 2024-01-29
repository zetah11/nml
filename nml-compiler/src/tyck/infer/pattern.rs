use crate::errors::ErrorId;
use crate::names::Name;
use crate::source::Span;
use crate::trees::{inferred as o, resolved as i};
use crate::tyck::{Checker, Scheme, Type};

impl<'a, 'lit> Checker<'a, '_, 'lit, '_> {
    /// When inferring patterns, we also keep track of a set of `wildcards` -
    /// unification variables generated by "matching anything"-patterns like
    /// wildcards and variable bindings. Concretely, given a set of patterns
    /// like `A -> _ | B -> _`, the inferred type of the pattern should be
    /// `A | B`, whereas a set of patterns like `A -> _ | B -> _ | _ -> _` can
    /// be given the type `A | B | 'a` (since the last case can match any
    /// variant).
    ///
    /// Once a set of patterns have an inferred type, it is minimized - all
    /// unsolved unification variables _not_ present in the `wildcards` set are
    /// unified with the empty row, to prevent their generalization.
    pub fn infer_pattern(
        &mut self,
        wildcards: &mut Vec<&'a Type<'a>>,
        pattern: &i::Pattern<'_, 'lit>,
    ) -> o::MonoPattern<'a> {
        let span = pattern.span;
        let (node, ty) = match &pattern.node {
            i::PatternNode::Invalid(e) => self.invalid_pattern(e),
            i::PatternNode::Wildcard => self.wildcard_pattern(wildcards),
            i::PatternNode::Unit => self.unit_pattern(),
            i::PatternNode::Bind(name) => self.bind_pattern(name, wildcards, span),
            i::PatternNode::Constructor(name) => self.constructor_pattern(name),

            i::PatternNode::Anno(pattern, ty) => {
                return self.anno_pattern(pattern, ty, wildcards, span)
            }

            i::PatternNode::Apply([ctr, arg]) => self.apply_pattern(wildcards, ctr, arg, span),
            i::PatternNode::Or([a, b]) => self.or_pattern(a, b, wildcards, span),
            i::PatternNode::And([a, b]) => self.and_pattern(a, b, wildcards, span),

            i::PatternNode::Group(pattern) => return self.infer_pattern(wildcards, pattern),
        };

        o::MonoPattern { node, span, ty }
    }

    /// ```types
    /// -------------
    /// <err> : <err>
    /// ```
    fn invalid_pattern(&mut self, e: &ErrorId) -> (o::MonoPatternNode<'a>, &'a Type<'a>) {
        (
            o::MonoPatternNode::Invalid(*e),
            &*self.alloc.alloc(Type::Invalid(*e)),
        )
    }

    /// ```types
    /// 'a fresh
    /// --------
    ///  _ : 'a
    /// ```
    fn wildcard_pattern(
        &mut self,
        wildcards: &mut Vec<&'a Type<'a>>,
    ) -> (o::MonoPatternNode<'a>, &'a Type<'a>) {
        (o::MonoPatternNode::Wildcard, self.wildcard_type(wildcards))
    }

    /// ```types
    /// ---------
    /// () : unit
    /// ```
    fn unit_pattern(&mut self) -> (o::MonoPatternNode<'a>, &'a Type<'a>) {
        (o::MonoPatternNode::Unit, &*self.alloc.alloc(Type::Unit))
    }

    /// ```types
    /// x : t in G    'a fresh
    /// ----------    --------
    /// G => x : t     x : 'a
    /// ```
    fn bind_pattern(
        &mut self,
        name: &Name,
        wildcards: &mut Vec<&'a Type<'a>>,
        span: Span,
    ) -> (o::MonoPatternNode<'a>, &'a Type<'a>) {
        let ty = self.wildcard_type(wildcards);

        // If this name has been given a type before, then this is
        // another branch in an or-pattern and the two types must be
        // equal.
        if let Some(prev) = self.env.try_lookup(name) {
            assert!(prev.is_mono());
            self.unify(span, prev.ty, ty);
        } else {
            self.env.insert(*name, Scheme::mono(ty));
        }

        (o::MonoPatternNode::Bind(*name), ty)
    }

    /// ```types
    ///    C : T in G
    /// ----------------
    /// G => C : inst(T)
    /// ```
    fn constructor_pattern(&mut self, name: &Name) -> (o::MonoPatternNode<'a>, &'a Type<'a>) {
        let ty = self.instantiate_name(name);
        (o::MonoPatternNode::Constructor(*name), ty)
    }

    /// ```types
    ///    G => a : t
    /// ----------------
    /// G => (a : t) : t
    /// ```
    fn anno_pattern(
        &mut self,
        pattern: &i::Pattern<'_, 'lit>,
        ty: &i::Type<'_, 'lit>,
        wildcards: &mut Vec<&'a Type<'a>>,
        span: Span,
    ) -> o::MonoPattern<'a> {
        let pattern = self.infer_pattern(wildcards, pattern);
        let ty = self.lower(ty);
        self.unify(span, ty, pattern.ty);
        pattern
    }

    /// ```types
    /// G => a1 : t1 -> t2   G => a2 : t1
    /// ---------------------------------
    ///          G => a1 a2 : t2
    /// ```
    fn apply_pattern(
        &mut self,
        wildcards: &mut Vec<&'a Type<'a>>,
        ctr: &i::Pattern<'_, 'lit>,
        arg: &i::Pattern<'_, 'lit>,
        span: Span,
    ) -> (o::MonoPatternNode<'a>, &'a Type<'a>) {
        let arrow = self.alloc.alloc(Type::Arrow);

        let ctr = self.infer_pattern(wildcards, ctr);
        let arg = self.infer_pattern(wildcards, arg);

        let res_ty = self.fresh();
        let fun_ty = self.alloc.alloc(Type::Apply(arrow, arg.ty));
        let fun_ty = self.alloc.alloc(Type::Apply(fun_ty, res_ty));
        self.unify(span, ctr.ty, fun_ty);

        let terms = self.alloc.alloc([ctr, arg]);
        (o::MonoPatternNode::Apply(terms), res_ty)
    }

    /// ```types
    /// G => a1 : t1   G => a2 : t1
    /// ---------------------------
    ///      G => a1 | a2 : t1
    /// ```
    fn or_pattern(
        &mut self,
        lhs: &i::Pattern<'_, 'lit>,
        rhs: &i::Pattern<'_, 'lit>,
        wildcards: &mut Vec<&'a Type<'a>>,
        span: Span,
    ) -> (o::MonoPatternNode<'a>, &'a Type<'a>) {
        let a = self.infer_pattern(wildcards, lhs);
        let b = self.infer_pattern(wildcards, rhs);

        let res_ty = a.ty;
        self.unify(span, a.ty, b.ty);

        let terms = self.alloc.alloc([a, b]);
        (o::MonoPatternNode::Or(terms), res_ty)
    }

    /// ```types
    /// G => a1 : t1   G => a2 : t1
    /// ---------------------------
    ///      G => a1 & a2 : t1
    /// ```
    fn and_pattern(
        &mut self,
        lhs: &i::Pattern<'_, 'lit>,
        rhs: &i::Pattern<'_, 'lit>,
        wildcards: &mut Vec<&'a Type<'a>>,
        span: Span,
    ) -> (o::MonoPatternNode<'a>, &'a Type<'a>) {
        let a = self.infer_pattern(wildcards, lhs);
        let b = self.infer_pattern(wildcards, rhs);

        let res_ty = a.ty;
        self.unify(span, a.ty, b.ty);

        let terms = self.alloc.alloc([a, b]);
        (o::MonoPatternNode::And(terms), res_ty)
    }

    /// Create a fresh type and add it as a wildcard type.
    fn wildcard_type(&mut self, wildcards: &mut Vec<&'a Type<'a>>) -> &'a Type<'a> {
        let ty = self.fresh();
        wildcards.push(ty);
        ty
    }
}
