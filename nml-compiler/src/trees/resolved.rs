use std::collections::BTreeMap;
use std::convert::Infallible;

use super::nodes;
use crate::errors::{ErrorId, Errors};
use crate::names::{Label, Name};
use crate::resolve::ItemId;
use crate::source::Span;

pub struct Program<'a> {
    pub items: &'a [&'a [Item<'a>]],
    pub defs: BTreeMap<Name, Span>,
    pub errors: Errors,
    pub unattached: Vec<(ErrorId, Span)>,
}

pub struct Data<'a>(std::marker::PhantomData<&'a ()>);

impl<'a> nodes::Data for Data<'a> {
    type Item = Item<'a>;
    type Expr = Expr<'a>;
    type Pattern = Pattern<'a>;

    type ExprName = Infallible;
    type PatternName = Infallible;
    type Var = Name;
    type Variant = Label;

    type Apply = (&'a Self::Expr, &'a Self::Expr);
}

pub struct Item<'a> {
    pub node: ItemNode<'a>,
    pub span: Span,
    pub id: ItemId,
}

pub struct Expr<'a> {
    pub node: ExprNode<'a>,
    pub span: Span,
}

pub struct Pattern<'a> {
    pub node: PatternNode<'a>,
    pub span: Span,
}

pub type ItemNode<'a> = nodes::ItemNode<Data<'a>>;
pub type ExprNode<'a> = nodes::ExprNode<'a, Data<'a>>;
pub type PatternNode<'a> = nodes::PatternNode<'a, Data<'a>>;
