use std::collections::BTreeMap;
use std::convert::Infallible;

use super::nodes;
use crate::errors::{ErrorId, Errors};
use crate::names::{Label, Name};
use crate::resolve::ItemId;
use crate::source::Span;

pub struct Program<'a, 'lit> {
    pub items: &'a [&'a [Item<'a, 'lit>]],
    pub defs: BTreeMap<Name, Span>,
    pub errors: Errors,
    pub unattached: Vec<(ErrorId, Span)>,
}

pub struct Data<'a, 'lit>(std::marker::PhantomData<&'a &'lit ()>);

impl<'a, 'lit> nodes::Data for Data<'a, 'lit> {
    type Item = Item<'a, 'lit>;
    type Expr = Expr<'a, 'lit>;
    type Pattern = Pattern<'a, 'lit>;

    type ExprName = Infallible;
    type PatternName = Infallible;
    type Var = Name;
    type Variant = Label;

    type Apply = &'a [Self::Expr; 2];
}

pub struct Item<'a, 'lit> {
    pub node: ItemNode<'a, 'lit>,
    pub span: Span,
    pub id: ItemId,
}

pub struct Expr<'a, 'lit> {
    pub node: ExprNode<'a, 'lit>,
    pub span: Span,
}

pub struct Pattern<'a, 'lit> {
    pub node: PatternNode<'a, 'lit>,
    pub span: Span,
}

pub type ItemNode<'a, 'lit> = nodes::ItemNode<Data<'a, 'lit>>;
pub type ExprNode<'a, 'lit> = nodes::ExprNode<'a, 'lit, Data<'a, 'lit>>;
pub type PatternNode<'a, 'lit> = nodes::PatternNode<'a, Data<'a, 'lit>>;
