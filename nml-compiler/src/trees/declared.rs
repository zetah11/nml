use std::collections::BTreeMap;
use std::convert::Infallible;

use super::{nodes, parsed, resolved};
use crate::names::{Ident, Name};
use crate::resolve::ItemId;
use crate::source::Span;

pub struct Data<'a, 'b, 'lit>(std::marker::PhantomData<(&'a &'lit (), &'b &'lit ())>);

impl<'a, 'b, 'lit> nodes::Data for Data<'a, 'b, 'lit> {
    type Item = Item<'a, 'b, 'lit>;
    type Expr = &'b parsed::Expr<'b, 'lit>;
    type Pattern = resolved::Pattern<'a, 'lit>;
    type Type = &'b parsed::Type<'b, 'lit>;

    type ExprName = Infallible;
    type PatternName = Infallible;
    type Var = Infallible;
    type Variant = Infallible;
    type Universal = Infallible;

    type Apply = Infallible;
    type GenScope = BTreeMap<Ident<'lit>, Name>;
}

pub struct Item<'a, 'b, 'lit> {
    pub node: ItemNode<'a, 'b, 'lit>,
    pub span: Span,
    pub id: ItemId,
}

pub type ItemNode<'a, 'b, 'lit> = nodes::ItemNode<Data<'a, 'b, 'lit>>;
