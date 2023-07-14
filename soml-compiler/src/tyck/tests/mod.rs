mod generalize;
mod rows;

use malachite::Integer;
use typed_arena::Arena;

use super::memory::Alloc;
use super::pretty::Pretty;
use super::tree::RecordRow;
use super::{Checker, Expr, Label, Name, Type};

struct Store<'a> {
    pub exprs: &'a Arena<Expr<'a>>,
    pub types: &'a Alloc<'a>,
}

impl<'a> Store<'a> {
    pub fn with<F, T>(f: F) -> T
    where
        F: for<'b, 'p> FnOnce(Store<'b>, Checker<'b, 'p>) -> T,
    {
        let _ = pretty_env_logger::try_init();
        let exprs = Arena::new();
        let types = Arena::new();
        let records = Arena::new();
        let alloc = Alloc::new(&types, &records);
        let mut pretty = Pretty::default().with_show_levels(true);
        let this = Store {
            exprs: &exprs,
            types: &alloc,
        };
        let checker = Checker::new(&alloc, &mut pretty);
        f(this, checker)
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

    pub fn if_then(
        &self,
        cond: &'a Expr<'a>,
        then: &'a Expr<'a>,
        elze: &'a Expr<'a>,
    ) -> &'a Expr<'a> {
        self.exprs.alloc(Expr::If(cond, then, elze))
    }

    pub fn field(&self, of: &'a Expr<'a>, label: impl Into<String>) -> &'a Expr<'a> {
        self.exprs.alloc(Expr::Field(of, Label::new(label)))
    }

    pub fn update(
        &self,
        label: impl Into<String>,
        value: &'a Expr<'a>,
        old: &'a Expr<'a>,
    ) -> &'a Expr<'a> {
        self.exprs
            .alloc(Expr::Extend(old, Label::new(label), value))
    }

    pub fn restrict(&self, expr: &'a Expr<'a>, label: impl Into<String>) -> &'a Expr<'a> {
        self.exprs.alloc(Expr::Restrict(expr, Label::new(label)))
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
        self.types.ty(Type::Fun(t, u))
    }

    pub fn boolean(&self) -> &'a Type<'a> {
        self.types.ty(Type::Boolean)
    }

    pub fn int(&self) -> &'a Type<'a> {
        self.types.ty(Type::Integer)
    }

    pub fn extend<L, I, Ii>(&self, fields: I, rest: &'a RecordRow<'a>) -> &'a Type<'a>
    where
        L: Into<String>,
        I: IntoIterator<Item = (L, &'a Type<'a>), IntoIter = Ii>,
        Ii: DoubleEndedIterator<Item = (L, &'a Type<'a>)>,
    {
        let mut rest = rest;
        for (label, field) in fields.into_iter().rev() {
            rest = self
                .types
                .record(RecordRow::Extend(Label::new(label), field, rest));
        }
        self.types.ty(Type::Record(rest))
    }
}
