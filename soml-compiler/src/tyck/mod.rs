pub use self::tree::{Env, ErrorId, Expr, Label, Name, Scheme, Type};

mod solve;
mod tree;

#[cfg(test)]
mod tests;

use typed_arena::Arena;

use self::solve::Solver;

pub struct Checker<'a> {
    types: &'a Arena<Type<'a>>,
    env: Env<'a>,
    solver: Solver<'a>,
}

impl<'a> Checker<'a> {
    pub fn new(types: &'a Arena<Type<'a>>) -> Self {
        Self {
            types,
            env: Env::new(),
            solver: Solver::new(),
        }
    }

    pub fn infer(&mut self, expr: &Expr) -> &'a Type<'a> {
        match expr {
            Expr::Invalid(e) => self.types.alloc(Type::Invalid(e.clone())),

            Expr::Var(name) => {
                let scheme = self.env.lookup(name);
                self.solver.instantiate(self.types, scheme)
            }

            Expr::Bool(_) => self.types.alloc(Type::Boolean),
            Expr::Number(_) => self.types.alloc(Type::Integer),

            Expr::If(cond, then, otherwise) => {
                let t1 = self.infer(cond);
                let t2 = self.infer(then);
                let t3 = self.infer(otherwise);

                self.solver
                    .unify(self.types, t1, self.types.alloc(Type::Boolean));
                self.solver.unify(self.types, t2, t3);
                t2
            }

            Expr::Field(record, label) => {
                let t = self.fresh();
                let r = self.fresh();
                let record_ty = self.types.alloc(Type::Extend(label.clone(), t, r));
                let record_ty = self.types.alloc(Type::Record(record_ty));
                let inferred = self.infer(record);
                self.solver.unify(self.types, inferred, record_ty);
                t
            }

            Expr::Empty => {
                let ty = self.types.alloc(Type::Empty);
                self.types.alloc(Type::Record(ty))
            }

            Expr::Extend(old, label, value) => {
                let r = self.fresh();

                let record_ty = self.types.alloc(Type::Record(r));
                let value_ty = self.infer(value);
                let inferred = self.infer(old);

                let ty = self.types.alloc(Type::Extend(label.clone(), value_ty, r));
                let ty = self.types.alloc(Type::Record(ty));

                self.solver.unify(self.types, inferred, record_ty);
                ty
            }

            Expr::Restrict(old, label) => {
                let t = self.fresh();
                let r = self.fresh();

                let record_ty = self.types.alloc(Type::Extend(label.clone(), t, r));
                let record_ty = self.types.alloc(Type::Record(record_ty));
                let inferred = self.infer(old);

                let ty = self.types.alloc(Type::Record(r));

                self.solver.unify(self.types, inferred, record_ty);
                ty
            }

            Expr::Apply(fun, arg) => {
                let fun_ty = self.infer(fun);
                let arg_ty = self.infer(arg);

                let u = self.fresh();
                let expected = self.types.alloc(Type::Fun(arg_ty, u));

                self.solver.unify(self.types, fun_ty, expected);
                u
            }

            Expr::Lambda(param, body) => {
                let t = self.fresh();
                let scheme = Scheme {
                    params: Vec::new(),
                    ty: t,
                };
                self.env.insert(param.clone(), scheme);
                let u = self.infer(body);
                self.types.alloc(Type::Fun(t, u))
            }

            Expr::Let(name, bound, body) => {
                let bound = self.enter(|this| this.infer(bound));
                let scheme = self.solver.generalize(self.types, bound);
                self.env.insert(name.clone(), scheme);
                self.infer(body)
            }
        }
    }

    pub fn apply(&self, ty: &'a Type<'a>) -> &'a Type<'a> {
        self.solver.apply(self.types, ty)
    }

    #[cfg(test)]
    fn assert_alpha_equal(&mut self, lhs: &'a Type<'a>, rhs: &'a Type<'a>) {
        if !alpha_equal(lhs, rhs) {
            let lhs = self.apply(lhs);
            let rhs = self.apply(rhs);
            panic!("Inequal types\n     {lhs:?}\nwith {rhs:?}");
        }
    }

    fn fresh(&mut self) -> &'a Type<'a> {
        self.solver.fresh(self.types)
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

#[cfg(test)]
fn alpha_equal<'a>(t: &'a Type<'a>, u: &'a Type<'a>) -> bool {
    use crate::tyck::solve::TypeVar;
    use std::collections::BTreeMap;

    fn inner<'a>(subst: &mut BTreeMap<TypeVar, TypeVar>, t: &'a Type<'a>, u: &'a Type<'a>) -> bool {
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

            (Type::Boolean, Type::Boolean)
            | (Type::Integer, Type::Integer)
            | (Type::Empty, Type::Empty) => true,

            (Type::Fun(t1, u1), Type::Fun(t2, u2)) => inner(subst, t1, t2) && inner(subst, u1, u2),

            (Type::Record(t), Type::Record(u)) => inner(subst, t, u),

            (Type::Extend(l1, f1, r1), Type::Extend(l2, f2, r2)) => {
                l1 == l2 && inner(subst, f1, f2) && inner(subst, r1, r2)
            }

            _ => false,
        }
    }

    inner(&mut BTreeMap::new(), t, u)
}
