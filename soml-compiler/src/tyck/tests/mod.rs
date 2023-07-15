mod generalize;
mod rows;

use std::cell::RefCell;
use std::collections::BTreeMap;

use lasso::ThreadedRodeo;
use malachite::Integer;
use typed_arena::Arena;

use crate::errors::Errors;
use crate::names::{Name, Names};
use crate::source::SourceId;

use super::memory::Alloc;
use super::pretty::Pretty;
use super::tree::{ExprNode, RecordRow};
use super::{Checker, Expr, Type};

struct Store<'a, 'ids> {
    pub exprs: &'a Arena<Expr<'a>>,
    pub types: &'a Alloc<'a>,
    pub names: RefCell<Names<'ids>>,
    pub source: SourceId,

    name_intern: RefCell<BTreeMap<String, Name>>,
}

impl<'a, 'ids> Store<'a, 'ids> {
    pub fn with<F, T>(f: F) -> T
    where
        F: for<'b, 'c, 'e, 'i, 'p> FnOnce(Store<'b, 'c>, Checker<'b, 'e, 'i, 'p>) -> T,
    {
        let _ = pretty_env_logger::try_init();
        let exprs = Arena::new();
        let types = Arena::new();
        let records = Arena::new();
        let alloc = Alloc::new(&types, &records);
        let ids = ThreadedRodeo::new();
        let source = SourceId::new(0);
        let this = Store {
            exprs: &exprs,
            types: &alloc,
            source,

            names: RefCell::new(Names::new(&ids, source)),
            name_intern: RefCell::new(BTreeMap::new()),
        };

        let mut errors = Errors::new();
        let mut pretty = Pretty::new(&ids).with_show_levels(true);

        let checker = Checker::new(&alloc, &mut errors, &mut pretty);

        f(this, checker)
    }

    pub fn bool(&self, value: bool) -> &'a Expr<'a> {
        let node = ExprNode::Bool(value);
        let span = self.source.span(0, 0);
        self.exprs.alloc(Expr { node, span })
    }

    pub fn num(&self, value: impl Into<Integer>) -> &'a Expr<'a> {
        let node = ExprNode::Number(value.into());
        let span = self.source.span(0, 0);
        self.exprs.alloc(Expr { node, span })
    }

    pub fn var(&self, name: impl Into<String>) -> &'a Expr<'a> {
        let name = self.name(name);
        let node = ExprNode::Var(name);
        let span = self.source.span(0, 0);
        self.exprs.alloc(Expr { node, span })
    }

    pub fn if_then(
        &self,
        cond: &'a Expr<'a>,
        then: &'a Expr<'a>,
        elze: &'a Expr<'a>,
    ) -> &'a Expr<'a> {
        let node = ExprNode::If(cond, then, elze);
        let span = self.source.span(0, 0);
        self.exprs.alloc(Expr { node, span })
    }

    pub fn field(&self, of: &'a Expr<'a>, label: impl AsRef<str>) -> &'a Expr<'a> {
        let label = self.names.borrow().label(label);
        let node = ExprNode::Field(of, label);
        let span = self.source.span(0, 0);
        self.exprs.alloc(Expr { node, span })
    }

    pub fn update(
        &self,
        label: impl AsRef<str>,
        value: &'a Expr<'a>,
        old: &'a Expr<'a>,
    ) -> &'a Expr<'a> {
        let label = self.names.borrow().label(label);
        let node = ExprNode::Extend(old, label, value);
        let span = self.source.span(0, 0);
        self.exprs.alloc(Expr { node, span })
    }

    pub fn restrict(&self, expr: &'a Expr<'a>, label: impl AsRef<str>) -> &'a Expr<'a> {
        let label = self.names.borrow().label(label);
        let node = ExprNode::Restrict(expr, label);
        let span = self.source.span(0, 0);
        self.exprs.alloc(Expr { node, span })
    }

    pub fn apply(&self, fun: &'a Expr<'a>, arg: &'a Expr<'a>) -> &'a Expr<'a> {
        let node = ExprNode::Apply(fun, arg);
        let span = self.source.span(0, 0);
        self.exprs.alloc(Expr { node, span })
    }

    pub fn lambda(&self, arg: impl Into<String>, body: &'a Expr<'a>) -> &'a Expr<'a> {
        let arg = self.name(arg);
        let node = ExprNode::Lambda(arg, body);
        let span = self.source.span(0, 0);
        self.exprs.alloc(Expr { node, span })
    }

    pub fn bind(
        &self,
        name: impl Into<String>,
        bound: &'a Expr<'a>,
        body: &'a Expr<'a>,
    ) -> &'a Expr<'a> {
        let name = self.name(name);
        let node = ExprNode::Let(name, bound, body);
        let span = self.source.span(0, 0);
        self.exprs.alloc(Expr { node, span })
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
        L: AsRef<str>,
        I: IntoIterator<Item = (L, &'a Type<'a>), IntoIter = Ii>,
        Ii: DoubleEndedIterator<Item = (L, &'a Type<'a>)>,
    {
        let mut rest = rest;
        for (label, field) in fields.into_iter().rev() {
            let label = self.names.borrow().label(label);
            rest = self.types.record(RecordRow::Extend(label, field, rest));
        }
        self.types.ty(Type::Record(rest))
    }

    fn name(&self, name: impl Into<String>) -> Name {
        let name = name.into();
        let mut interned = self.name_intern.borrow_mut();
        if let Some(name) = interned.get(&name) {
            *name
        } else {
            let mut names = self.names.borrow_mut();
            let id = names.intern(&name);
            let id = names.name(None, id);
            interned.insert(name, id);
            id
        }
    }
}
