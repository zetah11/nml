pub use self::types::{Env, Scheme, Type};

mod memory;
mod pretty;
mod solve;
mod tree;
mod types;

#[cfg(test)]
mod tests;

use log::trace;

use self::memory::Alloc;
use self::pretty::{Prettifier, Pretty};
use self::solve::Solver;
use self::tree::{Expr, ExprNode};
use self::types::Row;
use crate::errors::Errors;
use crate::source::Span;

struct Reporting<'a, 'b, 'c> {
    pretty: &'a mut Prettifier<'b, 'c>,
    errors: &'a mut Errors,
    at: Span,
}

pub struct Checker<'a, 'err, 'ids, 'p> {
    types: &'a Alloc<'a>,
    env: Env<'a>,
    solver: Solver<'a>,
    errors: &'err mut Errors,
    pretty: &'p mut Pretty<'ids>,
}

impl<'a, 'err, 'ids, 'p> Checker<'a, 'err, 'ids, 'p> {
    pub fn new(
        types: &'a Alloc<'a>,
        errors: &'err mut Errors,
        pretty: &'p mut Pretty<'ids>,
    ) -> Self {
        Self {
            types,
            env: Env::new(),
            solver: Solver::new(),
            errors,
            pretty,
        }
    }

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

            ExprNode::Case {
                scrutinee,
                cases,
                catchall,
            } => {
                let scrutinee_ty = self.infer(scrutinee);
                let result_ty = self.fresh();

                let mut row = if let Some((binding, then)) = catchall {
                    let row = self.fresh_row();
                    let arg_ty = self.types.ty(Type::Variant(row));
                    self.env.insert(*binding, Scheme::mono(arg_ty));
                    let then_ty = self.infer(then);

                    let mut pretty = self.pretty.build();
                    self.solver.unify(
                        &mut pretty,
                        self.types,
                        self.errors,
                        span,
                        result_ty,
                        then_ty,
                    );

                    row
                } else {
                    self.types.row(Row::Empty)
                };

                for (label, binding, then) in cases.iter().rev() {
                    let arg_ty = self.fresh();
                    self.env.insert(*binding, Scheme::mono(arg_ty));
                    let then_ty = self.infer(then);

                    let mut pretty = self.pretty.build();
                    self.solver.unify(
                        &mut pretty,
                        self.types,
                        self.errors,
                        span,
                        result_ty,
                        then_ty,
                    );

                    row = self.types.row(Row::Extend(*label, arg_ty, row));
                }

                let mut pretty = self.pretty.build();
                let row = self.types.ty(Type::Variant(row));
                self.solver.unify(
                    &mut pretty,
                    self.types,
                    self.errors,
                    span,
                    scrutinee_ty,
                    row,
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

    pub fn apply(&self, ty: &'a Type<'a>) -> &'a Type<'a> {
        self.solver.apply(self.types, ty)
    }

    fn fresh(&mut self) -> &'a Type<'a> {
        self.solver.fresh(self.types)
    }

    fn fresh_row(&mut self) -> &'a Row<'a> {
        self.solver.fresh_record(self.types)
    }

    fn enter<F, T>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.solver.enter();
        let result = f(self);
        self.solver.exit();
        result
    }

    #[cfg(test)]
    fn assert_alpha_equal(&mut self, lhs: &'a Type<'a>, rhs: &'a Type<'a>) {
        let lhs = self.apply(lhs);
        let rhs = self.apply(rhs);

        if !alpha_equal(lhs, rhs) {
            let mut pretty = self.pretty.build();
            let lhs = pretty.ty(self.solver.apply(self.types, lhs));
            let rhs = pretty.ty(self.solver.apply(self.types, rhs));

            panic!("Inequal types\n    {lhs}\nand {rhs}");
        }
    }
}

fn to_name(n: usize) -> String {
    if n == 0 {
        "a".into()
    } else {
        let mut n = n;
        let mut res = String::new();
        while n > 0 {
            let c = char::from_u32('a' as u32 + (n % 26) as u32)
                .expect("a + [0, 26) is always a lowercase letter");
            n /= 26;
            res.push(c);
        }
        res
    }
}

#[cfg(test)]
fn alpha_equal<'a>(t: &'a Type<'a>, u: &'a Type<'a>) -> bool {
    use crate::tyck::solve::TypeVar;
    use std::collections::BTreeMap;

    fn inner(subst: &mut BTreeMap<TypeVar, TypeVar>, t: &Type, u: &Type) -> bool {
        match (t, u) {
            (Type::Invalid(_), Type::Invalid(_)) => true,
            (Type::Var(v1, _), Type::Var(v2, _)) => {
                if let Some(v1) = subst.get(v1) {
                    v1 == v2
                } else if let Some(v2) = subst.get(v2) {
                    v1 == v2
                } else {
                    subst.insert(*v1, *v2);
                    subst.insert(*v2, *v1);
                    true
                }
            }

            (Type::Param(n), Type::Param(m)) => n == m,
            (Type::Boolean, Type::Boolean) | (Type::Integer, Type::Integer) => true,
            (Type::Fun(t1, u1), Type::Fun(t2, u2)) => inner(subst, t1, t2) && inner(subst, u1, u2),
            (Type::Record(r), Type::Record(s)) | (Type::Variant(r), Type::Variant(s)) => {
                inner_row(subst, r, s)
            }

            _ => false,
        }
    }

    fn inner_row(subst: &mut BTreeMap<TypeVar, TypeVar>, r: &Row, s: &Row) -> bool {
        match (r, s) {
            (Row::Invalid(_), Row::Invalid(_)) => true,
            (Row::Var(v1, _), Row::Var(v2, _)) => {
                if let Some(v1) = subst.get(v1) {
                    v1 == v2
                } else if let Some(v2) = subst.get(v2) {
                    v1 == v2
                } else {
                    subst.insert(*v1, *v2);
                    subst.insert(*v2, *v1);
                    true
                }
            }

            (Row::Param(n), Row::Param(m)) => n == m,
            (Row::Empty, Row::Empty) => true,
            (Row::Extend(l1, field1, rest1), Row::Extend(l2, field2, rest2)) => {
                l1 == l2 && inner(subst, field1, field2) && inner_row(subst, rest1, rest2)
            }

            _ => false,
        }
    }

    inner(&mut BTreeMap::new(), t, u)
}
