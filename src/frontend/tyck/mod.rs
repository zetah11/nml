pub use self::pretty::{Prettifier, Pretty};
pub use self::types::{Env, Generic, Row, Scheme, Type};

mod infer;
mod lower;
mod messages;
mod pattern;
mod pretty;
mod solve;
mod types;

#[cfg(test)]
mod tests;

use bumpalo::Bump;

use self::solve::Solver;
use crate::frontend::errors::{ErrorId, Errors};
use crate::frontend::names::{Name, Names};
use crate::frontend::source::Span;
use crate::frontend::trees::{inferred, resolved};

pub fn infer<'a, 'src>(
    alloc: &'a Bump,
    names: &'a Names<'src>,
    program: &resolved::Program<'_, 'src>,
) -> inferred::Program<'a, 'src> {
    let mut errors = program.errors.clone();
    let mut pretty = Pretty::new(names)
        .with_show_levels(false)
        .with_show_error_id(false);
    let mut checker = Checker::new(alloc, &mut errors, &mut pretty);

    let items =
        alloc.alloc_slice_fill_iter(program.items.iter().map(|items| checker.check_items(items)));

    inferred::Program {
        items,
        defs: program.defs.clone(),
        errors,
        unattached: program.unattached.clone(),
    }
}

struct Reporting<'a, 'b, 'c, 'd> {
    pretty: &'a mut Prettifier<'b, 'c, 'd>,
    errors: &'a mut Errors,
    at: Span,
}

struct Checker<'a, 'err, 'src, 'p> {
    alloc: &'a Bump,
    env: Env<'a>,
    solver: Solver<'a>,
    errors: &'err mut Errors,
    pretty: &'p mut Pretty<'a, 'src>,
    holes: Vec<(Span, &'a Type<'a>)>,
}

impl<'a, 'err, 'src, 'p> Checker<'a, 'err, 'src, 'p> {
    pub fn new(
        alloc: &'a Bump,
        errors: &'err mut Errors,
        pretty: &'p mut Pretty<'a, 'src>,
    ) -> Self {
        Self {
            alloc,
            env: Env::new(),
            solver: Solver::new(),
            errors,
            pretty,
            holes: Vec::new(),
        }
    }

    /// Check a set of mutually recursive items.
    pub fn check_items<'b>(
        &mut self,
        items: &'b [resolved::Item<'_, 'src>],
    ) -> &'a [inferred::Item<'a, 'src>] {
        let mut inferred_items = Vec::with_capacity(items.len());

        self.enter(|this| {
            let mut typed_items = Vec::with_capacity(items.len());

            // Bind each item to a fresh var
            for item in items {
                let node = match &item.node {
                    resolved::ItemNode::Invalid(e) => inferred::BoundItemNode::Invalid(*e),
                    resolved::ItemNode::Let(pattern, expr, scope) => {
                        let mut wildcards = Vec::new();
                        let pattern = this.infer_pattern(&mut wildcards, pattern);

                        let keep = wildcards
                            .into_iter()
                            .flat_map(|ty| this.vars_in_ty(ty))
                            .collect();

                        this.minimize(&keep, pattern.ty);

                        let scope = this
                            .alloc
                            .alloc_slice_fill_iter(scope.iter().copied().map(Generic::Ticked));

                        inferred::BoundItemNode::Let(pattern, expr, scope)
                    }

                    resolved::ItemNode::Data(pattern, body) => {
                        let ty = this.type_pattern(pattern);
                        let body = this.check_data(&ty, body);
                        inferred::BoundItemNode::Data(ty, body)
                    }
                };

                let item = inferred::BoundItem {
                    node,
                    span: item.span,
                    id: item.id,
                };
                typed_items.push(item);
            }

            // Infer the type of each item and unify with bound type
            for item in typed_items {
                let node = match item.node {
                    inferred::BoundItemNode::Invalid(e) => inferred::BoundItemNode::Invalid(e),
                    inferred::BoundItemNode::Let(pattern, body, scope) => {
                        let body = this.infer(body);
                        this.unify(item.span, pattern.ty, body.ty);
                        inferred::BoundItemNode::Let(pattern, body, scope)
                    }

                    inferred::BoundItemNode::Data(ty, body) => {
                        inferred::BoundItemNode::Data(ty, body)
                    }
                };

                inferred_items.push(inferred::BoundItem {
                    node,
                    span: item.span,
                    id: item.id,
                });
            }
        });

        // Generalize!
        self.alloc
            .alloc_slice_fill_iter(inferred_items.into_iter().map(|item| {
                let id = item.id;
                let span = item.span;
                let node = match item.node {
                    inferred::BoundItemNode::Invalid(e) => inferred::ItemNode::Invalid(e),
                    inferred::BoundItemNode::Let(pattern, expr, scope) => {
                        let pattern = self.generalize_pattern(scope, &pattern);
                        inferred::ItemNode::Let(pattern, expr, ())
                    }

                    inferred::BoundItemNode::Data(ty, body) => inferred::ItemNode::Data(ty, body),
                };

                inferred::Item { node, span, id }
            }))
    }

    fn type_pattern(&mut self, pat: &resolved::DataPattern) -> Scheme<'a> {
        fn get_names(pat: &resolved::DataPattern) -> Result<(Name, Vec<Name>), ErrorId> {
            let name = pat.name?;
            let args: Result<_, ErrorId> = pat.args.iter().copied().collect();
            Ok((name, args?))
        }

        match get_names(pat) {
            Ok((name, args)) => {
                let params: Vec<_> = args.into_iter().map(Generic::Ticked).collect();
                let mut ty = Type::Named(name);

                for arg in params.iter() {
                    ty = Type::Apply(self.alloc.alloc(ty), self.alloc.alloc(Type::Param(*arg)));
                }

                let ty = self.alloc.alloc(ty);

                Scheme { ty, params }
            }

            Err(e) => {
                let ty = self.alloc.alloc(Type::Invalid(e));
                Scheme::mono(ty)
            }
        }
    }

    fn check_data(
        &mut self,
        scheme: &Scheme<'a>,
        data: &resolved::Data<'_, 'src>,
    ) -> inferred::Data<'a> {
        let span = data.span;
        let node = match &data.node {
            resolved::DataNode::Invalid(e) => inferred::DataNode::Invalid(*e),
            resolved::DataNode::Sum(ctors) => {
                let ctors = self.alloc.alloc_slice_fill_iter(
                    ctors
                        .iter()
                        .map(|ctor| self.check_constructor(scheme, ctor)),
                );

                inferred::DataNode::Sum(ctors)
            }
        };

        inferred::Data { node, span }
    }

    fn check_constructor(
        &mut self,
        scheme: &Scheme<'a>,
        ctor: &resolved::Constructor<'_, 'src>,
    ) -> inferred::Constructor<'a> {
        let span = ctor.span;
        let node = match &ctor.node {
            resolved::ConstructorNode::Invalid(e) => inferred::ConstructorNode::Invalid(*e),
            resolved::ConstructorNode::Constructor(name, params) => {
                let arrow = self.alloc.alloc(Type::Arrow);
                let mut ty = scheme.ty;

                let params = self
                    .alloc
                    .alloc_slice_fill_iter(params.iter().map(|ty| self.lower(ty).clone()));

                for param in params.iter().rev() {
                    let apply = self.alloc.alloc(Type::Apply(arrow, param));
                    ty = self.alloc.alloc(Type::Apply(apply, ty));
                }

                self.env.insert(*name, scheme.onto(ty));

                inferred::ConstructorNode::Constructor(*name, params)
            }
        };

        inferred::Constructor { node, span }
    }

    #[cfg(test)]
    fn assert_alpha_equal(&mut self, lhs: &'a Type<'a>, rhs: &'a Type<'a>) {
        let lhs = self.apply(lhs);
        let rhs = self.apply(rhs);

        if !alpha_equal(lhs, rhs) {
            let mut pretty = self.pretty.build();
            let lhs = pretty.ty(lhs);
            let rhs = pretty.ty(rhs);

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
    use std::collections::BTreeMap;

    use crate::frontend::tyck::solve::TypeVar;

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
            (Type::Integer, Type::Integer) | (Type::Arrow, Type::Arrow) => true,

            (Type::Record(r), Type::Record(s)) | (Type::Variant(r), Type::Variant(s)) => {
                inner_row(subst, r, s)
            }

            (Type::Apply(t1, t2), Type::Apply(u1, u2)) => {
                inner(subst, t1, u1) && inner(subst, t2, u2)
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
