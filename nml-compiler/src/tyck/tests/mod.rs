mod generalize;
mod rows;
mod sums;

use std::cell::RefCell;
use std::collections::BTreeMap;

use bumpalo::Bump;
use internment::Arena;
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
        F: for<'b, 'c, 'e, 'p> FnOnce(Store<'b, 'c>, Checker<'b, 'e, 'c, 'p>) -> T,
    {
        let _ = pretty_env_logger::try_init();
        let alloc = Bump::new();
        let source = SourceId::new(0);
        let ids = Arena::new();
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

    pub fn num(&self, value: impl Into<Integer>) -> Expr<'a, 'ids> {
        let value = self.literals.intern(value.into()).into_ref();
        self.expr(ExprNode::Number(value))
    }

    pub fn var(&self, name: impl Into<String>) -> Expr<'a, 'ids> {
        let name = self.name(name);
        self.expr(ExprNode::Var(name))
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

    pub fn case<I>(&self, scrutinee: Expr<'a, 'ids>, cases: I) -> Expr<'a, 'ids>
    where
        I: IntoIterator<Item = (Pattern<'a, 'ids>, Expr<'a, 'ids>)>,
        I::IntoIter: ExactSizeIterator,
    {
        let cases = self.alloc.alloc_slice_fill_iter(cases);
        let lambda = self.expr(ExprNode::Lambda(cases));
        let terms = self.alloc.alloc([lambda, scrutinee]);
        self.expr(ExprNode::Apply(terms))
    }

    pub fn apply(&self, fun: Expr<'a, 'ids>, arg: Expr<'a, 'ids>) -> Expr<'a, 'ids> {
        let terms = self.alloc.alloc([fun, arg]);
        self.expr(ExprNode::Apply(terms))
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
        let terms = self.alloc.alloc([bound, body]);
        self.expr(ExprNode::Let(pattern, terms, self.alloc.alloc([])))
    }

    pub fn bind(&self, name: impl Into<String>) -> Pattern<'a, 'ids> {
        let name = self.name(name);
        self.pattern(PatternNode::Bind(name))
    }

    pub fn named(&self, name: impl Into<String>) -> Pattern<'a, 'ids> {
        let name = self.name(name);
        self.pattern(PatternNode::Constructor(name))
    }

    pub fn apply_pat(&self, ctr: Pattern<'a, 'ids>, arg: Pattern<'a, 'ids>) -> Pattern<'a, 'ids> {
        let terms = self.alloc.alloc([ctr, arg]);
        self.pattern(PatternNode::Apply(terms))
    }

    pub fn arrow(&self, t: &'a Type<'a>, u: &'a Type<'a>) -> &'a Type<'a> {
        let arrow = self.alloc.alloc(Type::Arrow);
        let ty = self.alloc.alloc(Type::Apply(arrow, t));
        self.alloc.alloc(Type::Apply(ty, u))
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
