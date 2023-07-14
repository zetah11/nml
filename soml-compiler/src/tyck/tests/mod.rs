mod generalize;

use malachite::Integer;
use typed_arena::Arena;

use super::{Expr, Name, Type};

struct Store<'a> {
    pub exprs: &'a Arena<Expr<'a>>,
    pub types: &'a Arena<Type<'a>>,
}

impl<'a> Store<'a> {
    pub fn with<F, T>(f: F) -> T
    where
        F: for<'b> FnOnce(Store<'b>) -> T,
    {
        let exprs = Arena::new();
        let types = Arena::new();
        f(Store {
            exprs: &exprs,
            types: &types,
        })
    }

    pub fn bool(&self, value: bool) -> &'a Expr<'a> {
        self.exprs.alloc(Expr::Bool(value))
    }

    pub fn num(&self, value: impl Into<Integer>) -> &'a Expr<'a> {
        self.exprs.alloc(Expr::Number(value.into()))
    }

    pub fn var(&self, name: impl Into<String>) -> &'a Expr<'a> {
        self.exprs.alloc(Expr::Var(Name::new(name)))
    }

    pub fn apply(&self, fun: &'a Expr<'a>, arg: &'a Expr<'a>) -> &'a Expr<'a> {
        self.exprs.alloc(Expr::Apply(fun, arg))
    }

    pub fn lambda(&self, arg: impl Into<String>, body: &'a Expr<'a>) -> &'a Expr<'a> {
        self.exprs.alloc(Expr::Lambda(Name::new(arg), body))
    }

    pub fn bind(
        &self,
        name: impl Into<String>,
        bound: &'a Expr<'a>,
        body: &'a Expr<'a>,
    ) -> &'a Expr<'a> {
        self.exprs.alloc(Expr::Let(Name::new(name), bound, body))
    }

    pub fn arrow(&self, t: &'a Type<'a>, u: &'a Type<'a>) -> &'a Type<'a> {
        self.types.alloc(Type::Fun(t, u))
    }

    pub fn boolean(&self) -> &'a Type<'a> {
        self.types.alloc(Type::Boolean)
    }

    pub fn int(&self) -> &'a Type<'a> {
        self.types.alloc(Type::Integer)
    }
}
