pub use self::types::{Env, Row, Scheme, Type};

mod infer;
mod pattern;
mod pretty;
mod solve;
mod types;

#[cfg(test)]
mod tests;

use bumpalo::Bump;

use self::pretty::{Prettifier, Pretty};
use self::solve::Solver;
use crate::errors::Errors;
use crate::names::Names;
use crate::source::Span;
use crate::trees::{inferred, resolved};

pub fn infer(names: &Names, program: &resolved::Program) -> Errors {
    let types = Bump::new();
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
    alloc: &'a Bump,
    env: Env<'a>,
    solver: Solver<'a>,
    errors: &'err mut Errors,
    pretty: &'p mut Pretty<'ids>,
    holes: Vec<(Span, &'a Type<'a>)>,
}

impl<'a, 'err, 'ids, 'p> Checker<'a, 'err, 'ids, 'p> {
    pub fn new(alloc: &'a Bump, errors: &'err mut Errors, pretty: &'p mut Pretty<'ids>) -> Self {
        Self { alloc, env: Env::new(), solver: Solver::new(), errors, pretty, holes: Vec::new() }
    }

    /// Check a set of mutually recursive items.
    pub fn check_items(&mut self, items: &[resolved::Item]) {
        let mut inferred_items = Vec::with_capacity(items.len());

        self.enter(|this| {
            // Bind each item to a fresh var
            let mut typed_items = Vec::with_capacity(items.len());
            for item in items {
                let ty = match &item.node {
                    resolved::ItemNode::Invalid(e) => self.alloc.alloc(Type::Invalid(*e)),
                    resolved::ItemNode::Let(name, (), _) => {
                        let ty = this.fresh();
                        if let Ok(name) = name {
                            this.env.insert(*name, Scheme::mono(ty));
                        }
                        ty
                    }
                };

                typed_items.push((item, ty));
            }

            // Infer the type of each item and unify with bound type
            for (item, ty) in typed_items {
                let (node, inferred) = match &item.node {
                    resolved::ItemNode::Invalid(e) => (inferred::ItemNode::Invalid(*e), ty),
                    resolved::ItemNode::Let(name, (), body) => {
                        let body = this.infer(body);
                        let ty = body.ty;
                        (inferred::ItemNode::Let(*name, (), body), ty)
                    }
                };

                let mut pretty = this.pretty.build();
                this.solver.unify(&mut pretty, this.alloc, this.errors, item.span, ty, inferred);

                inferred_items.push(inferred::Item { node, span: item.span, id: item.id });
            }
        });

        // Generalize!
        let mut pretty = self.pretty.build();
        for item in inferred_items {
            match &item.node {
                inferred::ItemNode::Invalid(_) => {}

                inferred::ItemNode::Let(name, (), _) => {
                    let Ok(name) = name else { continue; };
                    let scheme = self.env.lookup(name);
                    debug_assert!(scheme.is_mono());
                    let scheme = self.solver.generalize(&mut pretty, self.alloc, scheme.ty);
                    self.env.overwrite(*name, scheme);
                }
            }
        }
    }

    fn fresh(&mut self) -> &'a Type<'a> {
        self.solver.fresh(self.alloc)
    }

    fn fresh_row(&mut self) -> &'a Row<'a> {
        self.solver.fresh_record(self.alloc)
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
            let lhs = pretty.ty(self.solver.apply(self.alloc, lhs));
            let rhs = pretty.ty(self.solver.apply(self.alloc, rhs));

            panic!("Inequal types\n    {lhs}\nand {rhs}");
        }
    }

    #[cfg(test)]
    pub fn apply(&self, ty: &'a Type<'a>) -> &'a Type<'a> {
        self.solver.apply(self.alloc, ty)
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
