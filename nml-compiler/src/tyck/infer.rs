use log::trace;

use super::{Checker, Row, Scheme, Type};
use crate::trees::resolved::{Expr, ExprNode, Pattern, PatternNode};

impl<'a> Checker<'a, '_, '_, '_> {
    pub fn infer(&mut self, expr: &Expr) -> &'a Type<'a> {
        let span = expr.span;
        match &expr.node {
            ExprNode::Invalid(e) => {
                trace!("infer err");
                trace!("done err");
                self.types.alloc(Type::Invalid(*e))
            }

            ExprNode::Var(name) => {
                trace!("infer var");
                let scheme = self.env.lookup(name);
                let mut pretty = self.pretty.build();
                let ty = self.solver.instantiate(&mut pretty, self.types, scheme);
                trace!("done var");
                ty
            }

            ExprNode::Hole => {
                trace!("infer hole");
                let ty = self.fresh();
                self.holes.push((span, ty));
                trace!("done hole");
                ty
            }

            ExprNode::Unit => self.types.alloc(Type::Unit),

            ExprNode::Bool(_) => {
                trace!("infer bool");
                trace!("done bool");
                self.types.alloc(Type::Boolean)
            }
            ExprNode::Number(_) => {
                trace!("infer num");
                trace!("done num");
                self.types.alloc(Type::Integer)
            }

            ExprNode::If(cond, then, otherwise) => {
                trace!("infer if");
                let t1 = self.infer(cond);
                let t2 = self.infer(then);
                let t3 = self.infer(otherwise);
                let bool = self.types.alloc(Type::Boolean);

                let mut pretty = self.pretty.build();

                self.solver.unify(&mut pretty, self.types, self.errors, span, t1, bool);

                self.solver.unify(&mut pretty, self.types, self.errors, span, t2, t3);

                trace!("done if");

                t2
            }

            ExprNode::Field(record, label, _) => {
                trace!("infer field");
                let inferred = self.infer(record);

                let t = match label {
                    Ok(label) => {
                        let t = self.fresh();
                        let r = self.fresh_row();
                        let record_ty = self.types.alloc(Row::Extend(*label, t, r));
                        let record_ty = self.types.alloc(Type::Record(record_ty));

                        let mut pretty = self.pretty.build();
                        self.solver.unify(
                            &mut pretty,
                            self.types,
                            self.errors,
                            span,
                            inferred,
                            record_ty,
                        );

                        t
                    }

                    Err(e) => self.types.alloc(Type::Invalid(*e)),
                };

                trace!("done field");
                t
            }

            ExprNode::Record(fields, extend) => {
                trace!("infer record");
                let mut row = if let Some(extend) = extend {
                    let row = self.fresh_row();
                    let arg_ty = self.types.alloc(Type::Record(row));
                    let extend_ty = self.infer(extend);

                    let mut pretty = self.pretty.build();
                    self.solver.unify(
                        &mut pretty,
                        self.types,
                        self.errors,
                        span,
                        arg_ty,
                        extend_ty,
                    );

                    row
                } else {
                    self.types.alloc(Row::Empty)
                };

                for (label, label_span, field) in fields.iter().rev() {
                    let field_ty = self.infer(field);

                    match label {
                        Ok(label) => {
                            row = self.types.alloc(Row::Extend(*label, field_ty, row));
                        }

                        Err(e) => {
                            let t = self.types.alloc(Type::Invalid(*e));
                            let mut pretty = self.pretty.build();
                            self.solver.unify(
                                &mut pretty,
                                self.types,
                                self.errors,
                                *label_span,
                                field_ty,
                                t,
                            );
                        }
                    }
                }

                trace!("done record");
                self.types.alloc(Type::Record(row))
            }

            ExprNode::Restrict(old, label) => {
                trace!("infer restrict");
                let t = self.fresh();
                let r = self.fresh_row();

                let record_ty = self.types.alloc(Row::Extend(*label, t, r));
                let record_ty = self.types.alloc(Type::Record(record_ty));
                let inferred = self.infer(old);

                let ty = self.types.alloc(Type::Record(r));

                let mut pretty = self.pretty.build();

                self.solver.unify(&mut pretty, self.types, self.errors, span, inferred, record_ty);

                trace!("done restrict");
                ty
            }

            ExprNode::Variant(name) => {
                let arg_ty = self.fresh();
                let row_ty = self.fresh_row();
                let row_ty = self.types.alloc(Row::Extend(*name, arg_ty, row_ty));
                let row_ty = self.types.alloc(Type::Variant(row_ty));

                self.types.alloc(Type::Fun(arg_ty, row_ty))
            }

            ExprNode::Case { scrutinee, cases } => {
                let scrutinee_ty = self.infer(scrutinee);
                let result_ty = self.fresh();
                let case_ty = self.fresh();

                let mut wildcards = Vec::new();

                for (pattern, then) in cases.iter() {
                    let pattern_ty = self.infer_pattern(&mut wildcards, pattern);
                    let then_ty = self.infer(then);

                    let mut pretty = self.pretty.build();
                    self.solver.unify(
                        &mut pretty,
                        self.types,
                        self.errors,
                        span,
                        case_ty,
                        pattern_ty,
                    );

                    self.solver.unify(
                        &mut pretty,
                        self.types,
                        self.errors,
                        span,
                        result_ty,
                        then_ty,
                    );
                }

                let keep =
                    wildcards.into_iter().flat_map(|ty| self.solver.vars_in_ty(ty)).collect();

                let mut pretty = self.pretty.build();
                self.solver.minimize(&mut pretty, self.types, &keep, case_ty);

                self.solver.unify(
                    &mut pretty,
                    self.types,
                    self.errors,
                    span,
                    scrutinee_ty,
                    case_ty,
                );

                result_ty
            }

            ExprNode::Apply(fun, arg) => {
                trace!("infer apply");
                let fun_ty = self.infer(fun);
                let arg_ty = self.infer(arg);

                let u = self.fresh();
                let expected = self.types.alloc(Type::Fun(arg_ty, u));

                let mut pretty = self.pretty.build();

                self.solver.unify(&mut pretty, self.types, self.errors, span, fun_ty, expected);

                trace!("done apply");
                u
            }

            ExprNode::Lambda(pattern, body) => {
                trace!("infer lambda");
                let mut wildcards = Vec::new();
                let pattern_ty = self.infer_pattern(&mut wildcards, pattern);
                let keep =
                    wildcards.into_iter().flat_map(|ty| self.solver.vars_in_ty(ty)).collect();

                let mut pretty = self.pretty.build();
                self.solver.minimize(&mut pretty, self.types, &keep, pattern_ty);

                let u = self.infer(body);
                trace!("done lambda");
                self.types.alloc(Type::Fun(pattern_ty, u))
            }

            ExprNode::Let(name, (), bound, body) => {
                trace!("infer let");
                let bound = self.enter(|this| this.infer(bound));
                let mut pretty = self.pretty.build();
                let scheme = self.solver.generalize(&mut pretty, self.types, bound);
                if let Ok(name) = name {
                    self.env.insert(*name, scheme);
                }
                trace!("done let");
                self.infer(body)
            }

            ExprNode::Small(v) | ExprNode::Big(v) => match *v {},
        }
    }
}

impl<'a> Checker<'a, '_, '_, '_> {
    fn infer_pattern(
        &mut self,
        wildcards: &mut Vec<&'a Type<'a>>,
        pattern: &Pattern,
    ) -> &'a Type<'a> {
        let span = pattern.span;
        match &pattern.node {
            PatternNode::Invalid(e) => self.types.alloc(Type::Invalid(*e)),

            PatternNode::Wildcard => self.wildcard_type(wildcards),

            PatternNode::Unit => self.types.alloc(Type::Unit),

            PatternNode::Bind(name) => {
                let ty = self.wildcard_type(wildcards);
                self.env.insert(*name, Scheme::mono(ty));
                ty
            }

            PatternNode::Named(name) => {
                let scheme = self.env.lookup(name);
                let mut pretty = self.pretty.build();
                self.solver.instantiate(&mut pretty, self.types, scheme)
            }

            PatternNode::Deconstruct(label, pattern) => {
                let pattern_ty = self.infer_pattern(wildcards, pattern);
                let row_ty = self.fresh_row();
                let row_ty = self.types.alloc(Row::Extend(*label, pattern_ty, row_ty));
                self.types.alloc(Type::Variant(row_ty))
            }

            PatternNode::Apply(ctr, arg) => {
                let ctr_ty = self.infer_pattern(wildcards, ctr);
                let arg_ty = self.infer_pattern(wildcards, arg);

                let res_ty = self.fresh();
                let fun_ty = self.types.alloc(Type::Fun(arg_ty, res_ty));

                let mut pretty = self.pretty.build();
                self.solver.unify(&mut pretty, self.types, self.errors, span, ctr_ty, fun_ty);

                res_ty
            }

            PatternNode::Small(v) | PatternNode::Big(v) => match *v {},
        }
    }

    fn wildcard_type(&mut self, wildcards: &mut Vec<&'a Type<'a>>) -> &'a Type<'a> {
        let ty = self.fresh();
        wildcards.push(ty);
        ty
    }
}
