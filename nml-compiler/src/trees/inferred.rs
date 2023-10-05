use std::collections::BTreeMap;
use std::convert::Infallible;

use super::nodes;
use crate::errors::{ErrorId, Errors};
use crate::names::Name;
use crate::resolve::ItemId;
use crate::source::Span;
use crate::tyck::{Generic, Scheme, Type};

pub struct Program<'a, 'lit> {
    pub items: &'a [&'a [Item<'a, 'lit>]],
    pub defs: BTreeMap<Name, Span>,
    pub errors: Errors,
    pub unattached: Vec<(ErrorId, Span)>,
}

pub struct Data<'a, 'lit>(std::marker::PhantomData<&'a &'lit ()>);

pub struct MonoData<'a, 'lit>(std::marker::PhantomData<&'a &'lit ()>);

type Var = Name;
type GenScope = ();
type ApplyExpr<'a, 'lit> = &'a [Expr<'a, 'lit>; 2];
type ApplyPolyPattern<'a> = &'a [PolyPattern<'a>; 2];
type ApplyMonoPattern<'a> = &'a [MonoPattern<'a>; 2];

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
pub struct PolyPattern<'a> {
    pub node: PolyPatternNode<'a>,
    pub span: Span,
    pub scheme: Scheme<'a>,
}

/// A pattern with a not yet generalized type.
pub struct MonoPattern<'a> {
    pub node: MonoPatternNode<'a>,
    pub span: Span,
    pub ty: &'a Type<'a>,
}

pub type ItemNode<'a, 'lit> = nodes::ItemNode<Expr<'a, 'lit>, PolyPattern<'a>, GenScope>;

pub type ExprNode<'a, 'lit> = nodes::ExprNode<
    'a,
    'lit,
    Expr<'a, 'lit>,
    PolyPattern<'a>,
    Infallible,
    Infallible,
    Var,
    ApplyExpr<'a, 'lit>,
    GenScope,
>;

pub type PolyPatternNode<'a> =
    nodes::PatternNode<'a, PolyPattern<'a>, Infallible, Infallible, Var, ApplyPolyPattern<'a>>;

pub type MonoPatternNode<'a> =
    nodes::PatternNode<'a, MonoPattern<'a>, Infallible, Infallible, Var, ApplyMonoPattern<'a>>;

pub(crate) struct BoundItem<'a, E> {
    pub node: BoundItemNode<'a, E>,
    pub span: Span,
    pub id: ItemId,
}

type BoundGenScope<'a> = &'a [Generic];

pub(crate) type BoundItemNode<'a, E> = nodes::ItemNode<E, MonoPattern<'a>, BoundGenScope<'a>>;

/// The syntax tree of an item _after_ its pattern has been bound a type but_
/// _before_ its body has been inferred.
pub(crate) struct BoundData<'a, T>(std::marker::PhantomData<(&'a (), T)>);
