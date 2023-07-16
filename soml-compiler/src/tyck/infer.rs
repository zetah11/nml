use log::trace;

use super::tree::{Expr, ExprNode, Pattern, PatternNode};
use super::{Checker, Row, Scheme, Type};

impl<'a> Checker<'a, '_, '_, '_> {
    pub fn infer(&mut self, expr: &Expr) -> &'a Type<'a> {
        let span = expr.span;
        match &expr.node {
            ExprNode::Invalid(e) => {
                trace!("infer err");
                trace!("done err");
                self.types.ty(Type::Invalid(*e))
            }

            ExprNode::Var(name) => {
                trace!("infer var");
                let scheme = self.env.lookup(name);
                let mut pretty = self.pretty.build();
                let t = self.solver.instantiate(&mut pretty, self.types, scheme);
                trace!("done var");
                t
            }

            ExprNode::Bool(_) => {
                trace!("infer bool");
                trace!("done bool");
                self.types.ty(Type::Boolean)
            }
            ExprNode::Number(_) => {
                trace!("infer num");
                trace!("done num");
                self.types.ty(Type::Integer)
            }

            ExprNode::If(cond, then, otherwise) => {
                trace!("infer if");
                let t1 = self.infer(cond);
                let t2 = self.infer(then);
                let t3 = self.infer(otherwise);
                let bool = self.types.ty(Type::Boolean);

                let mut pretty = self.pretty.build();

                self.solver
                    .unify(&mut pretty, self.types, self.errors, span, t1, bool);

                self.solver
                    .unify(&mut pretty, self.types, self.errors, span, t2, t3);

                trace!("done if");

                t2
            }

            ExprNode::Field(record, label) => {
                trace!("infer field");
                let t = self.fresh();
                let r = self.fresh_row();
                let record_ty = self.types.row(Row::Extend(*label, t, r));
                let record_ty = self.types.ty(Type::Record(record_ty));
                let inferred = self.infer(record);

                let mut pretty = self.pretty.build();

                self.solver.unify(
                    &mut pretty,
                    self.types,
                    self.errors,
                    span,
                    inferred,
                    record_ty,
                );

                trace!("done field");

                t
            }

            ExprNode::Record(fields, extend) => {
                trace!("infer record");
                let mut row = if let Some(extend) = extend {
                    let row = self.fresh_row();
                    let arg_ty = self.types.ty(Type::Record(row));
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
                    self.types.row(Row::Empty)
                };

                for (label, field) in fields.iter().rev() {
                    let field_ty = self.infer(field);
                    row = self.types.row(Row::Extend(*label, field_ty, row));
                }

                trace!("done record");
                self.types.ty(Type::Record(row))
            }

            ExprNode::Restrict(old, label) => {
                trace!("infer restrict");
                let t = self.fresh();
                let r = self.fresh_row();

                let record_ty = self.types.row(Row::Extend(*label, t, r));
                let record_ty = self.types.ty(Type::Record(record_ty));
                let inferred = self.infer(old);

                let ty = self.types.ty(Type::Record(r));

                let mut pretty = self.pretty.build();

                self.solver.unify(
                    &mut pretty,
                    self.types,
                    self.errors,
                    span,
                    inferred,
                    record_ty,
                );

                trace!("done restrict");
                ty
            }

            ExprNode::Variant(name) => {
                let arg_ty = self.fresh();
                let row_ty = self.fresh_row();
                let row_ty = self.types.row(Row::Extend(*name, arg_ty, row_ty));
                let row_ty = self.types.ty(Type::Variant(row_ty));

                self.types.ty(Type::Fun(arg_ty, row_ty))
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

                let keep = wildcards
                    .into_iter()
                    .flat_map(|ty| self.solver.vars_in_ty(ty))
                    .collect();

                let mut pretty = self.pretty.build();
                self.solver
                    .minimize(&mut pretty, self.types, &keep, case_ty);

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
                let expected = self.types.ty(Type::Fun(arg_ty, u));

                let mut pretty = self.pretty.build();

                self.solver
                    .unify(&mut pretty, self.types, self.errors, span, fun_ty, expected);

                trace!("done apply");
                u
            }

            ExprNode::Lambda(param, body) => {
                trace!("infer lambda");
                let t = self.fresh();
                let scheme = Scheme::mono(t);
                self.env.insert(*param, scheme);
                let u = self.infer(body);
                trace!("done lambda");
                self.types.ty(Type::Fun(t, u))
            }

            ExprNode::Let(name, bound, body) => {
                trace!("infer let");
                let bound = self.enter(|this| this.infer(bound));
                let mut pretty = self.pretty.build();
                let scheme = self.solver.generalize(&mut pretty, self.types, bound);
                self.env.insert(*name, scheme);
                trace!("done let");
                self.infer(body)
            }
        }
    }
}

impl<'a> Checker<'a, '_, '_, '_> {
    fn infer_pattern(
        &mut self,
        wildcards: &mut Vec<&'a Type<'a>>,
        pattern: &Pattern,
    ) -> &'a Type<'a> {
        match &pattern.node {
            PatternNode::Invalid(e) => self.types.ty(Type::Invalid(*e)),

            PatternNode::Wildcard => self.wildcard_type(wildcards),

            PatternNode::Bind(name) => {
                let ty = self.wildcard_type(wildcards);
                self.env.insert(*name, Scheme::mono(ty));
                ty
            }

            PatternNode::Deconstruct(label, pattern) => {
                let pattern_ty = self.infer_pattern(wildcards, pattern);
                let row_ty = self.fresh_row();
                let row_ty = self.types.row(Row::Extend(*label, pattern_ty, row_ty));
                self.types.ty(Type::Variant(row_ty))
            }
        }
    }

    fn wildcard_type(&mut self, wildcards: &mut Vec<&'a Type<'a>>) -> &'a Type<'a> {
        let ty = self.fresh();
        wildcards.push(ty);
        ty
    }
}
