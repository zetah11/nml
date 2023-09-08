use std::collections::BTreeMap;
use std::convert::Infallible;

use super::nodes;
use crate::errors::{ErrorId, Errors};
use crate::names::{Label, Name};
use crate::resolve::ItemId;
use crate::source::Span;
use crate::tyck::{Scheme, Type};

pub struct Program<'a, 'lit> {
    pub items: &'a [&'a [Item<'a, 'lit>]],
    pub defs: BTreeMap<Name, Span>,
    pub errors: Errors,
    pub unattached: Vec<(ErrorId, Span)>,
}

pub struct Data<'a, 'lit>(std::marker::PhantomData<&'a &'lit ()>);

pub struct MonoData<'a, 'lit>(std::marker::PhantomData<&'a &'lit ()>);

impl<'a, 'lit> nodes::Data for Data<'a, 'lit> {
    type Item = Item<'a, 'lit>;
    type Expr = Expr<'a, 'lit>;
    type Pattern = PolyPattern<'a, 'lit>;
    type Type = Infallible;

    type ExprName = Infallible;
    type PatternName = Infallible;
    type Var = Name;
    type Variant = Label<'lit>;

    type Apply = &'a [Self::Expr; 2];
    type GenScope = &'a [Name];
}

impl<'a, 'lit> nodes::Data for MonoData<'a, 'lit> {
    type Item = Infallible;
    type Expr = Infallible;
    type Pattern = MonoPattern<'a, 'lit>;
    type Type = Infallible;

    type ExprName = Infallible;
    type PatternName = Infallible;
    type Var = Name;
    type Variant = Label<'lit>;

    type Apply = Infallible;
    type GenScope = Infallible;
}

pub struct Item<'a, 'lit> {
    pub node: ItemNode<'a, 'lit>,
    pub span: Span,
    pub id: ItemId,
}

pub struct Expr<'a, 'lit> {
    pub node: ExprNode<'a, 'lit>,
    pub span: Span,
    pub ty: &'a Type<'a>,
}

/// A pattern with a generalized type.
pub struct PolyPattern<'a, 'lit> {
    pub node: PolyPatternNode<'a, 'lit>,
    pub span: Span,
    pub scheme: Scheme<'a>,
}

/// A pattern with a not yet generalized type.
pub struct MonoPattern<'a, 'lit> {
    pub node: MonoPatternNode<'a, 'lit>,
    pub span: Span,
    pub ty: &'a Type<'a>,
}

pub type ItemNode<'a, 'lit> = nodes::ItemNode<Data<'a, 'lit>>;
pub type ExprNode<'a, 'lit> = nodes::ExprNode<'a, 'lit, Data<'a, 'lit>>;
pub type PolyPatternNode<'a, 'lit> = nodes::PatternNode<'a, Data<'a, 'lit>>;
pub type MonoPatternNode<'a, 'lit> = nodes::PatternNode<'a, MonoData<'a, 'lit>>;

pub(crate) struct BoundItem<'a, 'lit, T> {
    pub node: BoundItemNode<'a, 'lit, T>,
    pub span: Span,
    pub id: ItemId,
}

pub(crate) type BoundItemNode<'a, 'lit, T> = nodes::ItemNode<BoundData<'a, 'lit, T>>;

/// The syntax tree of an item _after_ its pattern has been bound a type but_
/// _before_ its body has been inferred.
pub(crate) struct BoundData<'a, 'lit, T>(std::marker::PhantomData<(&'a &'lit (), T)>);

impl<'a, 'lit, T> nodes::Data for BoundData<'a, 'lit, T> {
    type Item = BoundItem<'a, 'lit, T>;
    type Expr = T;
    type Pattern = MonoPattern<'a, 'lit>;
    type Type = Infallible;

    type ExprName = Infallible;
    type PatternName = Infallible;
    type Var = Name;
    type Variant = Name;

    type Apply = Infallible;
    type GenScope = &'a [Name];
}
