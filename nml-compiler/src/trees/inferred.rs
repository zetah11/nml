//! An inferred program is one where each node in the syntax tree is annotated
//! with its type.

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

pub struct DataBody<'a>(pub &'a [DataConstructor<'a>]);

pub struct DataConstructor<'a> {
    pub name: Result<Name, ErrorId>,
    pub params: &'a [Type<'a>],
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

type TypeSyntax = Infallible;
type TypePattern<'a> = &'a Type<'a>;
type Var = Name;
type Constructor = Name;
type ApplyExpr<'a, 'lit> = &'a [Expr<'a, 'lit>; 2];
type ApplyPolyPattern<'a> = &'a [PolyPattern<'a>; 2];
type ApplyMonoPattern<'a> = &'a [MonoPattern<'a>; 2];
type GenScope = ();

pub type ItemNode<'a, 'lit> =
    nodes::ItemNode<Expr<'a, 'lit>, PolyPattern<'a>, TypePattern<'a>, DataBody<'a>, GenScope>;

pub type ExprNode<'a, 'lit> = nodes::ExprNode<
    'a,
    'lit,
    Expr<'a, 'lit>,
    PolyPattern<'a>,
    TypeSyntax,
    Var,
    ApplyExpr<'a, 'lit>,
    GenScope,
>;

pub type PolyPatternNode<'a> =
    nodes::PatternNode<'a, PolyPattern<'a>, TypeSyntax, Var, Constructor, ApplyPolyPattern<'a>>;

pub type MonoPatternNode<'a> =
    nodes::PatternNode<'a, MonoPattern<'a>, TypeSyntax, Var, Constructor, ApplyMonoPattern<'a>>;

pub(crate) struct BoundItem<'a, E> {
    pub node: BoundItemNode<'a, E>,
    pub span: Span,
    pub id: ItemId,
}

type BoundGenScope<'a> = &'a [Generic];

pub(crate) type BoundItemNode<'a, E> =
    nodes::ItemNode<E, MonoPattern<'a>, TypePattern<'a>, DataBody<'a>, BoundGenScope<'a>>;
