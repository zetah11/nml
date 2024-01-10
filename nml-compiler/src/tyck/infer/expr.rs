use log::trace;
use malachite::Integer;

use crate::errors::ErrorId;
use crate::names::{Label, Name};
use crate::source::Span;
use crate::trees::{inferred as o, resolved as i};
use crate::tyck::Generic;
use crate::tyck::{Checker, Row, Type};

impl<'a, 'lit> Checker<'a, '_, 'lit, '_> {
    pub fn infer(&mut self, expr: &i::Expr<'_, 'lit>) -> o::Expr<'a, 'lit> {
        let span = expr.span;
        let (node, ty) = match &expr.node {
            i::ExprNode::Invalid(e) => self.invalid_expr(e),
            i::ExprNode::Var(name) => self.var(name),
            i::ExprNode::Hole => self.hole(span),
            i::ExprNode::Unit => self.unit(),
            i::ExprNode::Number(v) => self.number(v),
            i::ExprNode::Anno(expr, ty) => return self.anno(expr, ty, span),

            i::ExprNode::Field(record, label, label_span) => {
                self.field(record, label, span, label_span)
            }

            i::ExprNode::Record(fields, extend) => self.record(fields, extend, span),
            i::ExprNode::Restrict(old, label) => self.restrict(old, label, span),
            i::ExprNode::Lambda(arrows) => self.lambda(arrows),
            i::ExprNode::Apply([fun, arg]) => self.infer_apply(fun, arg, span),

            i::ExprNode::Let(pattern, [bound, body], scope) => {
                self.infer_let(pattern, bound, body, scope)
            }

            i::ExprNode::Group(expr) => return self.infer(expr),
        };

        o::Expr { node, span, ty }
    }

    /// ```types
    /// <err> : <err>
    /// ```
    fn invalid_expr(&mut self, e: &ErrorId) -> (o::ExprNode<'a, 'lit>, &'a Type<'a>) {
        trace!("infer err");
        trace!("done err");
        (
            o::ExprNode::Invalid(*e),
            &*self.alloc.alloc(Type::Invalid(*e)),
        )
    }

    /// ```types
    ///    x : T in G
    /// ----------------
    /// G => x : inst(T)
    /// ```
    fn var(&mut self, name: &Name) -> (o::ExprNode<'a, 'lit>, &'a Type<'a>) {
        trace!("infer var");
        let ty = self.instantiate_name(name);
        let ty = &*self.alloc.alloc(ty);
        trace!("done var");
        (o::ExprNode::Var(*name), ty)
    }

    /// ```types
    /// 'a fresh
    /// --------
    ///  _ : 'a
    /// ```
    fn hole(&mut self, span: Span) -> (o::ExprNode<'a, 'lit>, &'a Type<'a>) {
        trace!("infer hole");
        let ty = self.fresh();
        self.holes.push((span, ty));
        trace!("done hole");
        (o::ExprNode::Hole, ty)
    }

    /// ```types
    /// ---------
    /// () : unit
    /// ```
    fn unit(&mut self) -> (o::ExprNode<'a, 'lit>, &'a Type<'a>) {
        (o::ExprNode::Unit, &*self.alloc.alloc(Type::Unit))
    }

    /// ```types
    /// -------
    /// n : int
    /// ```
    fn number(&mut self, v: &'lit Integer) -> (o::ExprNode<'a, 'lit>, &'a Type<'a>) {
        trace!("infer num");
        trace!("done num");
        (o::ExprNode::Number(v), &*self.alloc.alloc(Type::Integer))
    }

    /// ```types
    ///    G => e : t
    /// ----------------
    /// G => (e : t) : t
    /// ```
    fn anno(
        &mut self,
        expr: &i::Expr<'_, 'lit>,
        ty: &i::Type<'_, 'lit>,
        span: Span,
    ) -> o::Expr<'a, 'lit> {
        trace!("infer anno");
        let expr = self.infer(expr);
        let ty = self.lower(ty);
        self.unify(span, ty, expr.ty);
        trace!("done anno");
        expr
    }

    /// ```types
    /// G => e : { f : 'a | r }
    /// -----------------------
    ///      G => e.f : 'a
    /// ```
    fn field(
        &mut self,
        record: &i::Expr<'_, 'lit>,
        label: &Result<Label<'lit>, ErrorId>,
        span: Span,
        label_span: &Span,
    ) -> (o::ExprNode<'a, 'lit>, &'a Type<'a>) {
        trace!("infer field");
        let record = self.infer(record);
        let record = self.alloc.alloc(record);

        let (label, ty) = match label {
            Ok(label) => {
                let t = self.fresh();
                let r = self.fresh_row();
                let record_ty = self.alloc.alloc(Row::Extend(*label, t, r));
                let record_ty = self.alloc.alloc(Type::Record(record_ty));
                self.unify(span, record.ty, record_ty);
                (Ok(*label), t)
            }

            Err(e) => (Err(*e), &*self.alloc.alloc(Type::Invalid(*e))),
        };

        trace!("done field");

        (o::ExprNode::Field(record, label, *label_span), ty)
    }

    /// ```types
    ///              G => e1 : t1   ...   G => eN : tN
    /// ----------------------------------------------------------
    /// G => { f1 = e1, ..., fN = eN } : { f1 : t1, ..., fN : tN }
    /// ```
    fn record(
        &mut self,
        fields: &[(Result<Label<'lit>, ErrorId>, Span, i::Expr<'_, 'lit>)],
        extend: &Option<&i::Expr<'_, 'lit>>,
        span: Span,
    ) -> (o::ExprNode<'a, 'lit>, &'a Type<'a>) {
        trace!("infer record");
        let (extend, mut row) = if let Some(extend) = extend {
            let row = self.fresh_row();
            let arg_ty = self.alloc.alloc(Type::Record(row));
            let extend = self.infer(extend);
            let extend = &*self.alloc.alloc(extend);
            self.unify(span, arg_ty, extend.ty);
            (Some(extend), row)
        } else {
            (None, &*self.alloc.alloc(Row::Empty))
        };

        let fields = self.alloc.alloc_slice_fill_iter(fields.iter().rev().map(
            |(label, label_span, field)| {
                let field = self.infer(field);

                match label {
                    Ok(label) => {
                        row = self.alloc.alloc(Row::Extend(*label, field.ty, row));
                        (Ok(*label), *label_span, field)
                    }

                    Err(e) => {
                        let t = self.alloc.alloc(Type::Invalid(*e));
                        self.unify(*label_span, field.ty, t);
                        (Err(*e), *label_span, field)
                    }
                }
            },
        ));

        fields.reverse();

        trace!("done record");
        (
            o::ExprNode::Record(fields, extend),
            &*self.alloc.alloc(Type::Record(row)),
        )
    }

    /// ```types
    /// G => e : { f : t | r }
    /// ----------------------
    ///     G => e \ f : r
    /// ```
    fn restrict(
        &mut self,
        old: &i::Expr<'_, 'lit>,
        label: &Label<'lit>,
        span: Span,
    ) -> (o::ExprNode<'a, 'lit>, &'a Type<'a>) {
        trace!("infer restrict");
        let t = self.fresh();
        let r = self.fresh_row();

        let record_ty = self.alloc.alloc(Row::Extend(*label, t, r));
        let record_ty = self.alloc.alloc(Type::Record(record_ty));
        let old = self.infer(old);
        let old = self.alloc.alloc(old);
        self.unify(span, old.ty, record_ty);

        trace!("done restrict");
        (
            o::ExprNode::Restrict(old, *label),
            &*self.alloc.alloc(Type::Record(r)),
        )
    }

    /// ```types
    /// G => a1 : t1   G => e1 : t2   ...   G => aN : t1   G => eN : t2
    /// ---------------------------------------------------------------
    ///          G => a1 => e1 | ... | aN => eN : min(t1) -> t2
    /// ```
    fn lambda(
        &mut self,
        arrows: &[(i::Pattern<'_, 'lit>, i::Expr<'_, 'lit>)],
    ) -> (o::ExprNode<'a, 'lit>, &'a Type<'a>) {
        let mut wildcards = Vec::new();
        let input_ty = self.fresh();
        let output_ty = self.fresh();

        let arrows = self
            .alloc
            .alloc_slice_fill_iter(arrows.iter().map(|(pattern, body)| {
                let pattern = self.infer_pattern(&mut wildcards, pattern);
                let body = self.infer(body);

                self.unify(pattern.span, input_ty, pattern.ty);
                self.unify(body.span, output_ty, body.ty);

                let pattern = self.monomorphic(&pattern);
                (pattern, body)
            }));

        let keep = wildcards
            .into_iter()
            .flat_map(|ty| self.vars_in_ty(ty))
            .collect();

        self.minimize(&keep, input_ty);

        let arrow = self.alloc.alloc(Type::Arrow);
        let ty = self.alloc.alloc(Type::Apply(arrow, input_ty));
        let ty = &*self.alloc.alloc(Type::Apply(ty, output_ty));

        (o::ExprNode::Lambda(arrows), ty)
    }

    /// ```types
    /// G => e1 : t1 -> t2   G => e2 : t1
    /// ---------------------------------
    ///          G => e1 e2 : t2
    /// ```
    fn infer_apply(
        &mut self,
        fun: &i::Expr<'_, 'lit>,
        arg: &i::Expr<'_, 'lit>,
        span: Span,
    ) -> (o::ExprNode<'a, 'lit>, &'a Type<'a>) {
        trace!("infer apply");
        let fun = self.infer(fun);
        let arg = self.infer(arg);

        let u = self.fresh();
        let arrow = self.alloc.alloc(Type::Arrow);
        let expected = self.alloc.alloc(Type::Apply(arrow, arg.ty));
        let expected = &*self.alloc.alloc(Type::Apply(expected, u));
        self.unify(span, fun.ty, expected);
        trace!("done apply");

        let terms = self.alloc.alloc([fun, arg]);
        (o::ExprNode::Apply(terms), u)
    }

    /// ```types
    /// G => e1 : t1    G, x : gen(e1) => e2 : t2
    /// -----------------------------------------
    ///         G => let x = e1 in e2 : t2
    /// ```
    fn infer_let(
        &mut self,
        pattern: &i::Pattern<'_, 'lit>,
        bound: &i::Expr<'_, 'lit>,
        body: &i::Expr<'_, 'lit>,
        scope: &[Name],
    ) -> (o::ExprNode<'a, 'lit>, &'a Type<'a>) {
        trace!("infer let");
        let (pattern, bound) = self.enter(|this| {
            let bound = this.infer(bound);

            let mut wildcards = Vec::new();
            let pattern = this.infer_pattern(&mut wildcards, pattern);

            let keep = wildcards
                .into_iter()
                .flat_map(|ty| this.vars_in_ty(ty))
                .collect();

            this.minimize(&keep, pattern.ty);
            this.unify(pattern.span, pattern.ty, bound.ty);

            (pattern, bound)
        });

        let scope = self
            .alloc
            .alloc_slice_fill_iter(scope.iter().copied().map(Generic::Ticked));

        let pattern = self.generalize_pattern(scope, &pattern);

        trace!("done let");
        let body = self.infer(body);
        let ty = body.ty;
        let terms = self.alloc.alloc([bound, body]);
        (o::ExprNode::Let(pattern, terms, ()), ty)
    }
}
