use self::memory::Alloc;
use self::tree::RecordRow;
pub use self::tree::{Env, ErrorId, Expr, Label, Name, Scheme, Type};

mod memory;
mod pretty;
mod solve;
mod tree;

#[cfg(test)]
mod tests;

use log::trace;

use self::pretty::Pretty;
use self::solve::Solver;

pub struct Checker<'a, 'p> {
    types: &'a Alloc<'a>,
    env: Env<'a>,
    solver: Solver<'a>,
    pretty: &'p mut Pretty,
}

impl<'a, 'p> Checker<'a, 'p> {
    pub fn new(types: &'a Alloc<'a>, pretty: &'p mut Pretty) -> Self {
        Self {
            types,
            env: Env::new(),
            solver: Solver::new(),
            pretty,
        }
    }

    pub fn infer(&mut self, expr: &Expr) -> &'a Type<'a> {
        match expr {
            Expr::Invalid(e) => {
                trace!("infer err");
                trace!("done err");
                self.types.ty(Type::Invalid(e.clone()))
            }

            Expr::Var(name) => {
                trace!("infer var");
                let scheme = self.env.lookup(name);
                let mut pretty = self.pretty.build();
                let t = self.solver.instantiate(&mut pretty, self.types, scheme);
                trace!("done var");
                t
            }

            Expr::Bool(_) => {
                trace!("infer bool");
                trace!("done bool");
                self.types.ty(Type::Boolean)
            }
            Expr::Number(_) => {
                trace!("infer num");
                trace!("done num");
                self.types.ty(Type::Integer)
            }

            Expr::If(cond, then, otherwise) => {
                trace!("infer if");
                let t1 = self.infer(cond);
                let t2 = self.infer(then);
                let t3 = self.infer(otherwise);

                let mut pretty = self.pretty.build();
                self.solver
                    .unify(&mut pretty, self.types, t1, self.types.ty(Type::Boolean));
                self.solver.unify(&mut pretty, self.types, t2, t3);
                trace!("done if");

                t2
            }

            Expr::Field(record, label) => {
                trace!("infer field");
                let t = self.fresh();
                let r = self.fresh_record();
                let record_ty = self.types.record(RecordRow::Extend(label.clone(), t, r));
                let record_ty = self.types.ty(Type::Record(record_ty));
                let inferred = self.infer(record);

                let mut pretty = self.pretty.build();
                self.solver
                    .unify(&mut pretty, self.types, inferred, record_ty);
                trace!("done field");

                t
            }

            Expr::Empty => {
                trace!("infer empty");
                let ty = self.types.record(RecordRow::Empty);
                trace!("done empty");
                self.types.ty(Type::Record(ty))
            }

            Expr::Extend(old, label, value) => {
                trace!("infer extend");
                let r = self.fresh_record();

                let record_ty = self.types.ty(Type::Record(r));
                let value_ty = self.infer(value);
                let inferred = self.infer(old);

                let ty = self
                    .types
                    .record(RecordRow::Extend(label.clone(), value_ty, r));
                let ty = self.types.ty(Type::Record(ty));

                let mut pretty = self.pretty.build();
                self.solver
                    .unify(&mut pretty, self.types, inferred, record_ty);
                trace!("done extend");

                ty
            }

            Expr::Restrict(old, label) => {
                trace!("infer restrict");
                let t = self.fresh();
                let r = self.fresh_record();

                let record_ty = self.types.record(RecordRow::Extend(label.clone(), t, r));
                let record_ty = self.types.ty(Type::Record(record_ty));
                let inferred = self.infer(old);

                let ty = self.types.ty(Type::Record(r));

                let mut pretty = self.pretty.build();
                self.solver
                    .unify(&mut pretty, self.types, inferred, record_ty);
                trace!("done restrict");
                ty
            }

            Expr::Apply(fun, arg) => {
                trace!("infer apply");
                let fun_ty = self.infer(fun);
                let arg_ty = self.infer(arg);

                let u = self.fresh();
                let expected = self.types.ty(Type::Fun(arg_ty, u));

                let mut pretty = self.pretty.build();
                self.solver.unify(&mut pretty, self.types, fun_ty, expected);
                trace!("done apply");
                u
            }

            Expr::Lambda(param, body) => {
                trace!("infer lambda");
                let t = self.fresh();
                let scheme = Scheme {
                    params: Vec::new(),
                    ty: t,
                };
                self.env.insert(param.clone(), scheme);
                let u = self.infer(body);
                trace!("done lambda");
                self.types.ty(Type::Fun(t, u))
            }

            Expr::Let(name, bound, body) => {
                trace!("infer let");
                let bound = self.enter(|this| this.infer(bound));
                let mut pretty = self.pretty.build();
                let scheme = self.solver.generalize(&mut pretty, self.types, bound);
                self.env.insert(name.clone(), scheme);
                trace!("done let");
                self.infer(body)
            }
        }
    }

    pub fn apply(&self, ty: &'a Type<'a>) -> &'a Type<'a> {
        self.solver.apply(self.types, ty)
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

    fn fresh(&mut self) -> &'a Type<'a> {
        self.solver.fresh(self.types)
    }

    fn fresh_record(&mut self) -> &'a RecordRow<'a> {
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
            (Type::Record(r), Type::Record(s)) => inner_record(subst, r, s),

            _ => false,
        }
    }

    fn inner_record(subst: &mut BTreeMap<TypeVar, TypeVar>, r: &RecordRow, s: &RecordRow) -> bool {
        match (r, s) {
            (RecordRow::Invalid(_), RecordRow::Invalid(_)) => true,
            (RecordRow::Var(v1, _), RecordRow::Var(v2, _)) => {
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

            (RecordRow::Param(n), RecordRow::Param(m)) => n == m,
            (RecordRow::Empty, RecordRow::Empty) => true,
            (RecordRow::Extend(l1, field1, rest1), RecordRow::Extend(l2, field2, rest2)) => {
                l1 == l2 && inner(subst, field1, field2) && inner_record(subst, rest1, rest2)
            }

            _ => false,
        }
    }

    inner(&mut BTreeMap::new(), t, u)
}
