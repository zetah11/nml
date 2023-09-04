mod generalize;
mod rows;
mod sums;

use std::cell::RefCell;
use std::collections::BTreeMap;

use bumpalo::Bump;
use internment::Arena;
use lasso::ThreadedRodeo;
use malachite::Integer;

use crate::errors::Errors;
use crate::names::{Name, Names, ScopeName};
use crate::source::SourceId;

use super::pretty::Pretty;
use super::types::Row;
use super::{Checker, Type};
use crate::trees::resolved::{Expr, ExprNode, Pattern, PatternNode};

struct Store<'a, 'ids> {
    pub alloc: &'a Bump,
    pub names: &'a Names<'ids>,
    pub source: SourceId,

    literals: &'ids Arena<Integer>,
    name_intern: RefCell<BTreeMap<String, Name>>,
}

impl<'a, 'ids> Store<'a, 'ids> {
    pub fn with<F, T>(f: F) -> T
    where
        F: for<'b, 'c, 'e, 'i, 'p> FnOnce(Store<'b, 'c>, Checker<'b, 'e, 'i, 'p>) -> T,
    {
        let _ = pretty_env_logger::try_init();
        let alloc = Bump::new();
        let ids = ThreadedRodeo::new();
        let source = SourceId::new(0);
        let literals = Arena::new();
        let names = Names::new(&ids);
        let this = Store {
            alloc: &alloc,
            source,
            names: &names,

            literals: &literals,
            name_intern: RefCell::new(BTreeMap::new()),
        };

        let mut errors = Errors::new();
        let mut pretty = Pretty::new(&names)
            .with_show_levels(true)
            .with_show_error_id(true);

        let checker = Checker::new(&alloc, &mut errors, &mut pretty);
        f(this, checker)
    }

    pub fn bool(&self, value: bool) -> Expr<'a, 'ids> {
        self.expr(ExprNode::Bool(value))
    }

    pub fn num(&self, value: impl Into<Integer>) -> Expr<'a, 'ids> {
        let value = self.literals.intern(value.into()).into_ref();
        self.expr(ExprNode::Number(value))
    }

    pub fn var(&self, name: impl Into<String>) -> Expr<'a, 'ids> {
        let name = self.name(name);
        self.expr(ExprNode::Var(name))
    }

    pub fn if_then(
        &self,
        cond: Expr<'a, 'ids>,
        then: Expr<'a, 'ids>,
        elze: Expr<'a, 'ids>,
    ) -> Expr<'a, 'ids> {
        let cond = self.alloc.alloc(cond);
        let then = self.alloc.alloc(then);
        let elze = self.alloc.alloc(elze);
        self.expr(ExprNode::If(cond, then, elze))
    }

    pub fn field(&self, of: Expr<'a, 'ids>, label: impl AsRef<str>) -> Expr<'a, 'ids> {
        let of = self.alloc.alloc(of);
        let label = self.names.label(label);
        self.expr(ExprNode::Field(of, Ok(label), self.source.span(0, 0)))
    }

    pub fn record<L, I>(&self, fields: I, rest: Option<Expr<'a, 'ids>>) -> Expr<'a, 'ids>
    where
        L: AsRef<str>,
        I: IntoIterator<Item = (L, Expr<'a, 'ids>)>,
        I::IntoIter: ExactSizeIterator,
    {
        let rest = rest.map(|rest| &*self.alloc.alloc(rest));
        let fields = self
            .alloc
            .alloc_slice_fill_iter(fields.into_iter().map(|(label, field)| {
                let label = self.names.label(label);
                let span = self.source.span(0, 0);
                (Ok(label), span, field)
            }));

        self.expr(ExprNode::Record(fields, rest))
    }

    pub fn restrict(&self, expr: Expr<'a, 'ids>, label: impl AsRef<str>) -> Expr<'a, 'ids> {
        let expr = self.alloc.alloc(expr);
        let label = self.names.label(label);
        self.expr(ExprNode::Restrict(expr, label))
    }

    pub fn variant(&self, label: impl AsRef<str>) -> Expr<'a, 'ids> {
        let label = self.names.label(label);
        self.expr(ExprNode::Variant(label))
    }

    pub fn case<I>(&self, scrutinee: Expr<'a, 'ids>, cases: I) -> Expr<'a, 'ids>
    where
        I: IntoIterator<Item = (Pattern<'a, 'ids>, Expr<'a, 'ids>)>,
        I::IntoIter: ExactSizeIterator,
    {
        let scrutinee = self.alloc.alloc(scrutinee);
        let cases = self.alloc.alloc_slice_fill_iter(cases);
        let lambda = self.expr(ExprNode::Lambda(cases));
        let lambda = self.alloc.alloc(lambda);
        self.expr(ExprNode::Apply((lambda, scrutinee)))
    }

    pub fn apply(&self, fun: Expr<'a, 'ids>, arg: Expr<'a, 'ids>) -> Expr<'a, 'ids> {
        let fun = self.alloc.alloc(fun);
        let arg = self.alloc.alloc(arg);
        self.expr(ExprNode::Apply((fun, arg)))
    }

    pub fn lambda(&self, arg: Pattern<'a, 'ids>, body: Expr<'a, 'ids>) -> Expr<'a, 'ids> {
        let arrows = self
            .alloc
            .alloc_slice_fill_iter(std::iter::once((arg, body)));
        self.expr(ExprNode::Lambda(arrows))
    }

    pub fn let_in(
        &self,
        pattern: Pattern<'a, 'ids>,
        bound: Expr<'a, 'ids>,
        body: Expr<'a, 'ids>,
    ) -> Expr<'a, 'ids> {
        let bound = self.alloc.alloc(bound);
        let body = self.alloc.alloc(body);
        self.expr(ExprNode::Let(pattern, bound, body))
    }

    pub fn wildcard(&self) -> Pattern<'a, 'ids> {
        self.pattern(PatternNode::Wildcard)
    }

    pub fn bind(&self, name: impl Into<String>) -> Pattern<'a, 'ids> {
        let name = self.name(name);
        self.pattern(PatternNode::Bind(name))
    }

    pub fn named(&self, name: impl Into<String>) -> Pattern<'a, 'ids> {
        let name = self.name(name);
        self.pattern(PatternNode::Named(name))
    }

    pub fn deconstruct(
        &self,
        label: impl AsRef<str>,
        pattern: Pattern<'a, 'ids>,
    ) -> Pattern<'a, 'ids> {
        let label = self.names.label(label);
        let pattern = self.alloc.alloc(pattern);
        self.pattern(PatternNode::Deconstruct(label, pattern))
    }

    pub fn apply_pat(&self, ctr: Pattern<'a, 'ids>, arg: Pattern<'a, 'ids>) -> Pattern<'a, 'ids> {
        let ctr = self.alloc.alloc(ctr);
        let arg = self.alloc.alloc(arg);
        self.pattern(PatternNode::Apply(ctr, arg))
    }

    pub fn arrow(&self, t: &'a Type<'a>, u: &'a Type<'a>) -> &'a Type<'a> {
        self.alloc.alloc(Type::Fun(t, u))
    }

    pub fn boolean(&self) -> &'a Type<'a> {
        self.alloc.alloc(Type::Boolean)
    }

    pub fn int(&self) -> &'a Type<'a> {
        self.alloc.alloc(Type::Integer)
    }

    pub fn extend<L, I, Ii>(&self, fields: I, rest: Option<&'a Row<'a>>) -> &'a Type<'a>
    where
        L: AsRef<str>,
        I: IntoIterator<Item = (L, &'a Type<'a>), IntoIter = Ii>,
        Ii: DoubleEndedIterator<Item = (L, &'a Type<'a>)>,
    {
        let row = self.row(fields, rest);
        self.alloc.alloc(Type::Record(row))
    }

    pub fn sum<L, I, Ii>(&self, cases: I, rest: Option<&'a Row<'a>>) -> &'a Type<'a>
    where
        L: AsRef<str>,
        I: IntoIterator<Item = (L, &'a Type<'a>), IntoIter = Ii>,
        Ii: DoubleEndedIterator<Item = (L, &'a Type<'a>)>,
    {
        let row = self.row(cases, rest);
        self.alloc.alloc(Type::Variant(row))
    }

    pub fn nominal(&self, name: impl Into<String>) -> &'a Type<'a> {
        let name = self.name(name);
        self.alloc.alloc(Type::Named(name))
    }

    pub fn name(&self, name: impl Into<String>) -> Name {
        let name = name.into();
        let mut interned = self.name_intern.borrow_mut();
        if let Some(name) = interned.get(&name) {
            *name
        } else {
            let id = self.names.intern(&name);
            let id = self.names.name(ScopeName::TopLevel(self.source), id);
            interned.insert(name, id);
            id
        }
    }

    fn expr(&self, node: ExprNode<'a, 'ids>) -> Expr<'a, 'ids> {
        let span = self.source.span(0, 0);
        Expr { node, span }
    }

    fn pattern(&self, node: PatternNode<'a, 'ids>) -> Pattern<'a, 'ids> {
        let span = self.source.span(0, 0);
        Pattern { node, span }
    }

    fn row<L, I, Ii>(&self, labels: I, rest: Option<&'a Row<'a>>) -> &'a Row<'a>
    where
        L: AsRef<str>,
        I: IntoIterator<Item = (L, &'a Type<'a>), IntoIter = Ii>,
        Ii: DoubleEndedIterator<Item = (L, &'a Type<'a>)>,
    {
        let mut rest = rest.unwrap_or_else(|| self.alloc.alloc(Row::Empty));
        for (label, field) in labels.into_iter().rev() {
            let label = self.names.label(label);
            rest = self.alloc.alloc(Row::Extend(label, field, rest));
        }
        rest
    }
}
