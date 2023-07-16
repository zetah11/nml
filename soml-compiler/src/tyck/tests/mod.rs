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
use super::types::Row;
use super::{Checker, Type};
use crate::trees::resolved::{Expr, ExprNode, Pattern, PatternNode};

struct Store<'a, 'ids> {
    pub exprs: &'a Arena<Expr<'a>>,
    pub patterns: &'a Arena<Pattern<'a>>,
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
        let patterns = Arena::new();
        let types = Arena::new();
        let records = Arena::new();
        let alloc = Alloc::new(&types, &records);
        let ids = ThreadedRodeo::new();
        let source = SourceId::new(0);
        let this = Store {
            exprs: &exprs,
            patterns: &patterns,
            types: &alloc,
            source,

            names: RefCell::new(Names::new(&ids)),
            name_intern: RefCell::new(BTreeMap::new()),
        };

        let mut errors = Errors::new();
        let mut pretty = Pretty::new(&ids).with_show_levels(true).with_show_error_id(true);

        let checker = Checker::new(&alloc, &mut errors, &mut pretty);
        f(this, checker)
    }

    pub fn bool(&self, value: bool) -> &'a Expr<'a> {
        self.expr(ExprNode::Bool(value))
    }

    pub fn num(&self, value: impl Into<Integer>) -> &'a Expr<'a> {
        self.expr(ExprNode::Number(value.into()))
    }

    pub fn var(&self, name: impl Into<String>) -> &'a Expr<'a> {
        let name = self.name(name);
        self.expr(ExprNode::Var(name))
    }

    pub fn if_then(
        &self,
        cond: &'a Expr<'a>,
        then: &'a Expr<'a>,
        elze: &'a Expr<'a>,
    ) -> &'a Expr<'a> {
        self.expr(ExprNode::If(cond, then, elze))
    }

    pub fn field(&self, of: &'a Expr<'a>, label: impl AsRef<str>) -> &'a Expr<'a> {
        let label = self.names.borrow().label(label);
        self.expr(ExprNode::Field(of, label))
    }

    pub fn record<L, I>(&self, fields: I, rest: Option<&'a Expr<'a>>) -> &'a Expr<'a>
    where
        L: AsRef<str>,
        I: IntoIterator<Item = (L, &'a Expr<'a>)>,
    {
        let fields = fields
            .into_iter()
            .map(|(label, field)| (self.names.borrow().label(label), field))
            .collect();

        self.expr(ExprNode::Record(fields, rest))
    }

    pub fn restrict(&self, expr: &'a Expr<'a>, label: impl AsRef<str>) -> &'a Expr<'a> {
        let label = self.names.borrow().label(label);
        self.expr(ExprNode::Restrict(expr, label))
    }

    pub fn variant(&self, label: impl AsRef<str>) -> &'a Expr<'a> {
        let label = self.names.borrow().label(label);
        self.expr(ExprNode::Variant(label))
    }

    pub fn case<I>(&self, scrutinee: &'a Expr<'a>, cases: I) -> &'a Expr<'a>
    where
        I: IntoIterator<Item = (&'a Pattern<'a>, &'a Expr<'a>)>,
    {
        let cases = cases.into_iter().collect();
        self.expr(ExprNode::Case { scrutinee, cases })
    }

    pub fn apply(&self, fun: &'a Expr<'a>, arg: &'a Expr<'a>) -> &'a Expr<'a> {
        self.expr(ExprNode::Apply(fun, arg))
    }

    pub fn lambda(&self, arg: impl Into<String>, body: &'a Expr<'a>) -> &'a Expr<'a> {
        let arg = self.name(arg);
        self.expr(ExprNode::Lambda(arg, body))
    }

    pub fn let_in(
        &self,
        name: impl Into<String>,
        bound: &'a Expr<'a>,
        body: &'a Expr<'a>,
    ) -> &'a Expr<'a> {
        let name = self.name(name);
        self.expr(ExprNode::Let(name, bound, body))
    }

    pub fn wildcard(&self) -> &'a Pattern<'a> {
        self.pattern(PatternNode::Wildcard)
    }

    pub fn bind(&self, name: impl Into<String>) -> &'a Pattern<'a> {
        let name = self.name(name);
        self.pattern(PatternNode::Bind(name))
    }

    pub fn deconstruct(&self, label: impl AsRef<str>, pattern: &'a Pattern<'a>) -> &'a Pattern<'a> {
        let label = self.names.borrow().label(label);
        self.pattern(PatternNode::Deconstruct(label, pattern))
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

    pub fn extend<L, I, Ii>(&self, fields: I, rest: Option<&'a Row<'a>>) -> &'a Type<'a>
    where
        L: AsRef<str>,
        I: IntoIterator<Item = (L, &'a Type<'a>), IntoIter = Ii>,
        Ii: DoubleEndedIterator<Item = (L, &'a Type<'a>)>,
    {
        let row = self.row(fields, rest);
        self.types.ty(Type::Record(row))
    }

    pub fn sum<L, I, Ii>(&self, cases: I, rest: Option<&'a Row<'a>>) -> &'a Type<'a>
    where
        L: AsRef<str>,
        I: IntoIterator<Item = (L, &'a Type<'a>), IntoIter = Ii>,
        Ii: DoubleEndedIterator<Item = (L, &'a Type<'a>)>,
    {
        let row = self.row(cases, rest);
        self.types.ty(Type::Variant(row))
    }

    fn expr(&self, node: ExprNode<'a>) -> &'a Expr<'a> {
        let span = self.source.span(0, 0);
        self.exprs.alloc(Expr { node, span })
    }

    fn pattern(&self, node: PatternNode<'a>) -> &'a Pattern<'a> {
        let span = self.source.span(0, 0);
        self.patterns.alloc(Pattern { node, span })
    }

    fn row<L, I, Ii>(&self, labels: I, rest: Option<&'a Row<'a>>) -> &'a Row<'a>
    where
        L: AsRef<str>,
        I: IntoIterator<Item = (L, &'a Type<'a>), IntoIter = Ii>,
        Ii: DoubleEndedIterator<Item = (L, &'a Type<'a>)>,
    {
        let mut rest = rest.unwrap_or_else(|| self.types.row(Row::Empty));
        for (label, field) in labels.into_iter().rev() {
            let label = self.names.borrow().label(label);
            rest = self.types.row(Row::Extend(label, field, rest));
        }
        rest
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
