pub use self::vars::Level;

mod rows;
mod vars;

use std::collections::BTreeMap;

use log::trace;
use typed_arena::Arena;

use super::pretty::Prettifier;
use super::{to_name, ErrorId, Name, Scheme, Type};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TypeVar(usize);

pub struct Solver<'a> {
    subst: BTreeMap<TypeVar, &'a Type<'a>>,
    counter: usize,
    level: usize,
}

impl<'a> Solver<'a> {
    pub fn new() -> Self {
        Self {
            subst: BTreeMap::new(),
            counter: 0,
            level: 0,
        }
    }

    pub fn fresh(&mut self, alloc: &'a Arena<Type<'a>>) -> &'a Type<'a> {
        self.counter += 1;
        alloc.alloc(Type::Var(TypeVar(self.counter), Level::new(self.level)))
    }

    pub fn enter(&mut self) {
        self.level += 1;
    }

    pub fn exit(&mut self) {
        self.level -= 1;
    }
}

/// Apply
impl<'a> Solver<'a> {
    /// Apply the current substitution to the given type.
    pub fn apply(&self, alloc: &'a Arena<Type<'a>>, ty: &'a Type<'a>) -> &'a Type<'a> {
        match ty {
            Type::Invalid(_) | Type::Boolean | Type::Integer | Type::Empty | Type::Param(_) => ty,

            Type::Var(v, _) => {
                if let Some(ty) = self.subst.get(v) {
                    self.apply(alloc, ty)
                } else {
                    ty
                }
            }

            Type::Fun(t, u) => {
                let t = self.apply(alloc, t);
                let u = self.apply(alloc, u);
                alloc.alloc(Type::Fun(t, u))
            }

            Type::Record(t) => {
                let t = self.apply(alloc, t);
                alloc.alloc(Type::Record(t))
            }

            Type::Extend(label, field, rest) => {
                let field = self.apply(alloc, field);
                let rest = self.apply(alloc, rest);
                alloc.alloc(Type::Extend(label.clone(), field, rest))
            }
        }
    }
}

/// Instantiation
impl<'a> Solver<'a> {
    pub fn instantiate(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Arena<Type<'a>>,
        scheme: &Scheme<'a>,
    ) -> &'a Type<'a> {
        trace!(
            "ins {}",
            pretty.scheme(&Scheme {
                params: scheme.params.clone(),
                ty: self.apply(alloc, scheme.ty),
            })
        );

        let subst = scheme
            .params
            .iter()
            .map(|name| (name, self.fresh(alloc)))
            .collect();

        self.inst_ty(alloc, &subst, scheme.ty)
    }

    fn inst_ty(
        &self,
        alloc: &'a Arena<Type<'a>>,
        subst: &BTreeMap<&Name, &'a Type<'a>>,
        ty: &'a Type<'a>,
    ) -> &'a Type<'a> {
        match ty {
            Type::Invalid(_) | Type::Boolean | Type::Integer | Type::Empty => ty,

            Type::Var(v, _) => {
                if let Some(ty) = self.subst.get(v) {
                    self.inst_ty(alloc, subst, ty)
                } else {
                    ty
                }
            }

            Type::Param(n) => subst.get(n).copied().unwrap_or(ty),

            Type::Fun(t, u) => {
                let t = self.inst_ty(alloc, subst, t);
                let u = self.inst_ty(alloc, subst, u);
                alloc.alloc(Type::Fun(t, u))
            }

            Type::Record(t) => {
                let t = self.inst_ty(alloc, subst, t);
                alloc.alloc(Type::Record(t))
            }

            Type::Extend(label, field, rest) => {
                let field = self.inst_ty(alloc, subst, field);
                let rest = self.inst_ty(alloc, subst, rest);
                alloc.alloc(Type::Extend(label.clone(), field, rest))
            }
        }
    }
}

/// Generalization
impl<'a> Solver<'a> {
    pub fn generalize(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Arena<Type<'a>>,
        ty: &'a Type<'a>,
    ) -> Scheme<'a> {
        trace!("gen {}", pretty.ty(self.apply(alloc, ty)));

        let mut subst = BTreeMap::new();
        let ty = self.gen_ty(alloc, &mut subst, ty);
        let params = subst.into_iter().map(|(_, (_, name))| name).collect();
        Scheme { params, ty }
    }

    fn gen_ty(
        &mut self,
        alloc: &'a Arena<Type<'a>>,
        subst: &mut BTreeMap<TypeVar, (&'a Type<'a>, Name)>,
        ty: &'a Type<'a>,
    ) -> &'a Type<'a> {
        match ty {
            Type::Invalid(_) | Type::Param(_) | Type::Boolean | Type::Integer | Type::Empty => ty,

            Type::Var(v, level) => {
                if let Some(ty) = self.subst.get(v) {
                    self.gen_ty(alloc, subst, ty)
                } else if let Some((param, _)) = subst.get(v) {
                    param
                } else if level.can_generalize(self.level) {
                    let name = Name::new(to_name(v.0));
                    let param = alloc.alloc(Type::Param(name.clone()));
                    subst.insert(*v, (param, name));
                    param
                } else {
                    ty
                }
            }

            Type::Fun(t, u) => {
                let t = self.gen_ty(alloc, subst, t);
                let u = self.gen_ty(alloc, subst, u);
                alloc.alloc(Type::Fun(t, u))
            }

            Type::Record(t) => {
                let t = self.gen_ty(alloc, subst, t);
                alloc.alloc(Type::Record(t))
            }

            Type::Extend(label, field, rest) => {
                let field = self.gen_ty(alloc, subst, field);
                let rest = self.gen_ty(alloc, subst, rest);
                alloc.alloc(Type::Extend(label.clone(), field, rest))
            }
        }
    }
}

/// Unification
impl<'a> Solver<'a> {
    pub fn unify(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Arena<Type<'a>>,
        lhs: &'a Type<'a>,
        rhs: &'a Type<'a>,
    ) {
        trace!(
            "uni {}  ~  {}",
            pretty.ty(self.apply(alloc, lhs)),
            pretty.ty(self.apply(alloc, rhs))
        );
        self.unify_ty(pretty, alloc, lhs, rhs)
    }

    fn unify_ty(
        &mut self,
        pretty: &mut Prettifier,
        alloc: &'a Arena<Type<'a>>,
        lhs: &'a Type<'a>,
        rhs: &'a Type<'a>,
    ) {
        match (lhs, rhs) {
            (Type::Boolean, Type::Boolean) => {}
            (Type::Boolean, Type::Invalid(_)) | (Type::Invalid(_), Type::Boolean) => {}

            (Type::Integer, Type::Integer) => {}
            (Type::Integer, Type::Invalid(_)) | (Type::Invalid(_), Type::Integer) => {}

            (Type::Param(_), Type::Param(_)) => {}
            (Type::Param(_), Type::Invalid(_)) | (Type::Invalid(_), Type::Param(_)) => {}

            (Type::Fun(t1, u1), Type::Fun(t2, u2)) => {
                self.unify_ty(pretty, alloc, t1, t2);
                self.unify_ty(pretty, alloc, u1, u2);
            }
            (Type::Fun(t, u), e @ Type::Invalid(_)) | (e @ Type::Invalid(_), Type::Fun(t, u)) => {
                self.unify_ty(pretty, alloc, t, e);
                self.unify_ty(pretty, alloc, u, e);
            }

            (Type::Empty, Type::Empty) => {}
            (Type::Empty, Type::Invalid(_)) | (Type::Invalid(_), Type::Empty) => {}

            (Type::Record(row1), Type::Record(row2)) => self.unify_ty(pretty, alloc, row1, row2),
            (Type::Record(row), e @ Type::Invalid(_))
            | (e @ Type::Invalid(_), Type::Record(row)) => self.unify_ty(pretty, alloc, row, e),

            (Type::Extend(label, field_ty1, rest1), row2 @ Type::Extend(..)) => {
                let (field_ty2, rest2) = self.rewrite(pretty, alloc, row2, label);
                // todo occurs check here!
                self.unify_ty(pretty, alloc, field_ty1, field_ty2);
                self.unify_ty(pretty, alloc, rest1, rest2);
            }

            (Type::Extend(_, field_ty, rest), e @ Type::Invalid(_))
            | (e @ Type::Invalid(_), Type::Extend(_, field_ty, rest)) => {
                self.unify_ty(pretty, alloc, field_ty, e);
                self.unify_ty(pretty, alloc, rest, e);
            }

            (Type::Var(var, level), ty) | (ty, Type::Var(var, level)) => {
                if let Some(rhs) = self.subst.get(var) {
                    self.unify_ty(pretty, alloc, ty, rhs)
                } else {
                    self.set(alloc, var, level, ty)
                }
            }

            (Type::Invalid(_), Type::Invalid(_)) => {}

            // Use the exhaustiveness check to ensure termination when unifying
            // with error types
            (
                Type::Boolean
                | Type::Integer
                | Type::Param(_)
                | Type::Fun(..)
                | Type::Empty
                | Type::Record(_)
                | Type::Extend(..),
                Type::Boolean
                | Type::Integer
                | Type::Param(_)
                | Type::Fun(..)
                | Type::Empty
                | Type::Record(_)
                | Type::Extend(..),
            ) => {
                let e = alloc.alloc(Type::Invalid(ErrorId::new("inequal types")));
                self.unify_ty(pretty, alloc, lhs, e);
                self.unify_ty(pretty, alloc, rhs, e);
            }
        }
    }

    fn set(&mut self, alloc: &'a Arena<Type<'a>>, var: &TypeVar, level: &Level, ty: &'a Type<'a>) {
        if let Type::Var(v, l2) = ty {
            l2.set_min(level);
            if v == var {
                return;
            }
        }

        // Occurs check
        if self.occurs(var, level, ty) {
            let ty = alloc.alloc(Type::Invalid(ErrorId::new("recursive type")));
            return self.set(alloc, var, level, ty);
        }

        let prev = self.subst.insert(*var, ty);
        debug_assert!(prev.is_none(), "overwrote previous unification variable");
    }

    fn occurs(&self, var: &TypeVar, l1: &Level, ty: &Type) -> bool {
        match ty {
            Type::Invalid(_) | Type::Boolean | Type::Integer | Type::Param(_) | Type::Empty => {
                false
            }

            Type::Var(war, l2) => {
                if let Some(ty) = self.subst.get(war) {
                    self.occurs(var, l1, ty)
                } else {
                    l2.set_min(l1);
                    var == war
                }
            }
            Type::Record(t) => self.occurs(var, l1, t),
            Type::Fun(t, u) | Type::Extend(_, t, u) => {
                self.occurs(var, l1, t) || self.occurs(var, l1, u)
            }
        }
    }
}
