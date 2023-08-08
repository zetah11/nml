mod pattern;

use log::trace;

use super::{Checker, Row, Scheme, Type};
use crate::trees::{inferred as o, resolved as i};

impl<'a> Checker<'a, '_, '_, '_> {
    pub fn infer(&mut self, expr: &i::Expr) -> o::Expr<'a> {
        let span = expr.span;
        let (node, ty) = match &expr.node {
            i::ExprNode::Invalid(e) => {
                trace!("infer err");
                trace!("done err");
                (o::ExprNode::Invalid(*e), &*self.alloc.alloc(Type::Invalid(*e)))
            }

            i::ExprNode::Var(name) => {
                trace!("infer var");
                let scheme = self.env.lookup(name);
                let mut pretty = self.pretty.build();
                let ty = self.solver.instantiate(&mut pretty, self.alloc, scheme);
                trace!("done var");
                (o::ExprNode::Var(*name), ty)
            }

            i::ExprNode::Hole => {
                trace!("infer hole");
                let ty = self.fresh();
                self.holes.push((span, ty));
                trace!("done hole");
                (o::ExprNode::Hole, ty)
            }

            i::ExprNode::Unit => (o::ExprNode::Unit, &*self.alloc.alloc(Type::Unit)),

            i::ExprNode::Bool(v) => {
                trace!("infer bool");
                trace!("done bool");
                (o::ExprNode::Bool(*v), &*self.alloc.alloc(Type::Boolean))
            }

            i::ExprNode::Number(v) => {
                trace!("infer num");
                trace!("done num");
                (o::ExprNode::Number(v.clone()), &*self.alloc.alloc(Type::Integer))
            }

            i::ExprNode::If(cond, then, otherwise) => {
                trace!("infer if");
                let cond = self.infer(cond);
                let cond = self.alloc.alloc(cond);
                let then = self.infer(then);
                let then = self.alloc.alloc(then);
                let elze = self.infer(otherwise);
                let elze = self.alloc.alloc(elze);

                let bool_ty = self.alloc.alloc(Type::Boolean);

                let mut pretty = self.pretty.build();

                self.solver.unify(&mut pretty, self.alloc, self.errors, span, cond.ty, bool_ty);
                self.solver.unify(&mut pretty, self.alloc, self.errors, span, then.ty, elze.ty);

                trace!("done if");

                (o::ExprNode::If(cond, then, elze), then.ty)
            }

            i::ExprNode::Field(record, label, label_span) => {
                trace!("infer field");
                let record = self.infer(record);
                let record = self.alloc.alloc(record);

                let (label, ty) = match label {
                    Ok(label) => {
                        let t = self.fresh();
                        let r = self.fresh_row();
                        let record_ty = self.alloc.alloc(Row::Extend(*label, t, r));
                        let record_ty = self.alloc.alloc(Type::Record(record_ty));

                        let mut pretty = self.pretty.build();
                        self.solver.unify(
                            &mut pretty,
                            self.alloc,
                            self.errors,
                            span,
                            record.ty,
                            record_ty,
                        );

                        (Ok(*label), t)
                    }

                    Err(e) => (Err(*e), &*self.alloc.alloc(Type::Invalid(*e))),
                };

                trace!("done field");
                (o::ExprNode::Field(record, label, *label_span), ty)
            }

            i::ExprNode::Record(fields, extend) => {
                trace!("infer record");
                let (extend, mut row) = if let Some(extend) = extend {
                    let row = self.fresh_row();
                    let arg_ty = self.alloc.alloc(Type::Record(row));
                    let extend = self.infer(extend);
                    let extend = &*self.alloc.alloc(extend);

                    let mut pretty = self.pretty.build();
                    self.solver.unify(
                        &mut pretty,
                        self.alloc,
                        self.errors,
                        span,
                        arg_ty,
                        extend.ty,
                    );

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
                                let mut pretty = self.pretty.build();
                                self.solver.unify(
                                    &mut pretty,
                                    self.alloc,
                                    self.errors,
                                    *label_span,
                                    field.ty,
                                    t,
                                );

                                (Err(*e), *label_span, field)
                            }
                        }
                    },
                ));

                fields.reverse();

                trace!("done record");
                (o::ExprNode::Record(fields, extend), &*self.alloc.alloc(Type::Record(row)))
            }

            i::ExprNode::Restrict(old, label) => {
                trace!("infer restrict");
                let t = self.fresh();
                let r = self.fresh_row();

                let record_ty = self.alloc.alloc(Row::Extend(*label, t, r));
                let record_ty = self.alloc.alloc(Type::Record(record_ty));
                let old = self.infer(old);
                let old = self.alloc.alloc(old);

                let mut pretty = self.pretty.build();

                self.solver.unify(&mut pretty, self.alloc, self.errors, span, old.ty, record_ty);

                trace!("done restrict");
                (o::ExprNode::Restrict(old, *label), &*self.alloc.alloc(Type::Record(r)))
            }

            i::ExprNode::Variant(name) => {
                trace!("infer variant");
                let arg_ty = self.fresh();
                let row_ty = self.fresh_row();
                let row_ty = self.alloc.alloc(Row::Extend(*name, arg_ty, row_ty));
                let row_ty = self.alloc.alloc(Type::Variant(row_ty));
                trace!("done variant");

                (o::ExprNode::Variant(*name), &*self.alloc.alloc(Type::Fun(arg_ty, row_ty)))
            }

            i::ExprNode::Case { scrutinee, cases } => {
                trace!("infer case");
                let scrutinee = self.infer(scrutinee);
                let scrutinee = self.alloc.alloc(scrutinee);
                let result_ty = self.fresh();

                let case_ty = self.fresh_row();
                let case_ty = self.alloc.alloc(Type::Variant(case_ty));

                let mut wildcards = Vec::new();

                let cases =
                    self.alloc.alloc_slice_fill_iter(cases.iter().map(|(pattern, then)| {
                        let pattern = self.infer_pattern(&mut wildcards, pattern);
                        let then = self.infer(then);

                        let mut pretty = self.pretty.build();
                        self.solver.unify(
                            &mut pretty,
                            self.alloc,
                            self.errors,
                            span,
                            case_ty,
                            pattern.ty,
                        );

                        self.solver.unify(
                            &mut pretty,
                            self.alloc,
                            self.errors,
                            span,
                            result_ty,
                            then.ty,
                        );

                        let pattern = self.monomorphic(&pattern);
                        (pattern, then)
                    }));

                let keep =
                    wildcards.into_iter().flat_map(|ty| self.solver.vars_in_ty(ty)).collect();

                let mut pretty = self.pretty.build();
                trace!("case: minimizing pattern types");
                self.solver.minimize(&mut pretty, self.alloc, &keep, case_ty);
                trace!("case: unifying scrutinee with case types");
                self.solver.unify(
                    &mut pretty,
                    self.alloc,
                    self.errors,
                    span,
                    scrutinee.ty,
                    case_ty,
                );

                trace!("done case");

                (o::ExprNode::Case { scrutinee, cases }, result_ty)
            }

            i::ExprNode::Apply(fun, arg) => {
                trace!("infer apply");
                let fun = self.infer(fun);
                let fun = self.alloc.alloc(fun);
                let arg = self.infer(arg);
                let arg = self.alloc.alloc(arg);

                let u = self.fresh();
                let expected = self.alloc.alloc(Type::Fun(arg.ty, u));

                let mut pretty = self.pretty.build();

                self.solver.unify(&mut pretty, self.alloc, self.errors, span, fun.ty, expected);

                trace!("done apply");
                (o::ExprNode::Apply(fun, arg), u)
            }

            i::ExprNode::Lambda(pattern, body) => {
                trace!("infer lambda");
                let mut wildcards = Vec::new();
                let pattern = self.infer_pattern(&mut wildcards, pattern);
                let pattern_ty = pattern.ty;
                let pattern = self.monomorphic(&pattern);

                let keep =
                    wildcards.into_iter().flat_map(|ty| self.solver.vars_in_ty(ty)).collect();

                let mut pretty = self.pretty.build();
                self.solver.minimize(&mut pretty, self.alloc, &keep, pattern_ty);

                let body = self.infer(body);
                let body = self.alloc.alloc(body);
                trace!("done lambda");

                let ty = &*self.alloc.alloc(Type::Fun(pattern_ty, body.ty));
                (o::ExprNode::Lambda(pattern, body), ty)
            }

            i::ExprNode::Let(pattern, bound, body) => {
                trace!("infer let");
                let (pattern, bound) = self.enter(|this| {
                    let bound = this.infer(bound);
                    let bound = self.alloc.alloc(bound);

                    let mut wildcards = Vec::new();
                    let pattern = this.infer_pattern(&mut wildcards, pattern);

                    let keep =
                        wildcards.into_iter().flat_map(|ty| this.solver.vars_in_ty(ty)).collect();

                    let mut pretty = this.pretty.build();
                    this.solver.minimize(&mut pretty, this.alloc, &keep, pattern.ty);
                    this.solver.unify(
                        &mut pretty,
                        this.alloc,
                        this.errors,
                        pattern.span,
                        pattern.ty,
                        bound.ty,
                    );

                    (pattern, bound)
                });

                let pattern = self.generalize(&pattern);

                trace!("done let");
                let body = self.infer(body);
                let body = self.alloc.alloc(body);
                (o::ExprNode::Let(pattern, bound, body), body.ty)
            }

            i::ExprNode::Small(v) | i::ExprNode::Big(v) => match *v {},
        };

        o::Expr { node, span, ty }
    }
}
