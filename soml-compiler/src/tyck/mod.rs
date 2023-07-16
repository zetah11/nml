pub use self::types::{Env, Row, Scheme, Type};

mod infer;
mod memory;
mod pretty;
mod solve;
mod types;

#[cfg(test)]
mod tests;

use typed_arena::Arena;

use self::memory::Alloc;
use self::pretty::{Prettifier, Pretty};
use self::solve::Solver;
use crate::errors::Errors;
use crate::names::Names;
use crate::source::Span;
use crate::trees::resolved::{Item, ItemNode, Program};

pub fn infer(names: &Names, program: &Program) -> Errors {
    let types = Arena::new();
    let rows = Arena::new();
    let types = Alloc::new(&types, &rows);
    let mut errors = program.errors.clone();
    let mut pretty = Pretty::new(names).with_show_levels(false).with_show_error_id(false);
    let mut checker = Checker::new(&types, &mut errors, &mut pretty);

    for items in program.items {
        checker.check_items(items);
    }

    errors
}

struct Reporting<'a, 'b, 'c> {
    pretty: &'a mut Prettifier<'b, 'c>,
    errors: &'a mut Errors,
    at: Span,
}

struct Checker<'a, 'err, 'ids, 'p> {
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
        Self { types, env: Env::new(), solver: Solver::new(), errors, pretty }
    }

    /// Check a set of mutually recursive items.
    pub fn check_items(&mut self, items: &[Item<'a>]) {
        let mut inferred_items = Vec::with_capacity(items.len());

        self.enter(|this| {
            // Bind each item to a fresh var
            let mut typed_items = Vec::with_capacity(items.len());
            for item in items {
                let ty = match &item.node {
                    ItemNode::Let(name, _) => {
                        let ty = this.fresh();
                        this.env.insert(*name, Scheme::mono(ty));
                        ty
                    }
                };

                typed_items.push((item, ty));
            }

            // Infer the type of each item and unify with bound type
            for (item, ty) in typed_items {
                let inferred = match &item.node {
                    ItemNode::Let(_, body) => this.infer(body),
                };

                let mut pretty = this.pretty.build();
                this.solver.unify(&mut pretty, this.types, this.errors, item.span, ty, inferred);

                inferred_items.push(item);
            }
        });

        // Generalize!
        let mut pretty = self.pretty.build();
        for item in inferred_items {
            match &item.node {
                ItemNode::Let(name, _) => {
                    let scheme = self.env.lookup(name);
                    debug_assert!(scheme.is_mono());
                    let scheme = self.solver.generalize(&mut pretty, self.types, scheme.ty);
                    self.env.overwrite(*name, scheme);
                }
            }
        }
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

    #[cfg(test)]
    pub fn apply(&self, ty: &'a Type<'a>) -> &'a Type<'a> {
        self.solver.apply(self.types, ty)
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

            (Type::Named(n), Type::Named(m)) => n == m,
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
