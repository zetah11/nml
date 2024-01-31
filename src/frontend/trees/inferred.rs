//! An inferred program is one where each node in the syntax tree is annotated
//! with its type.

use std::collections::BTreeMap;
use std::convert::Infallible;

use super::nodes;
use crate::frontend::errors::{ErrorId, Errors};
use crate::frontend::names::Name;
use crate::frontend::resolve::ItemId;
use crate::frontend::source::Span;
use crate::frontend::tyck::{Generic, Scheme, Type};

pub struct Program<'a, 'src> {
    pub items: &'a [&'a [Item<'a, 'src>]],
    pub defs: BTreeMap<Name, Span>,
    pub errors: Errors,
    pub unattached: Vec<(ErrorId, Span)>,
}

pub struct Item<'a, 'src> {
    pub node: ItemNode<'a, 'src>,
    pub span: Span,
    pub id: ItemId,
}

pub struct Expr<'a, 'src> {
    pub node: ExprNode<'a, 'src>,
    pub span: Span,
    pub ty: &'a Type<'a>,
}

pub struct Data<'a> {
    pub node: DataNode<'a>,
    pub span: Span,
}

pub struct Constructor<'a> {
    pub node: ConstructorNode<'a>,
    pub span: Span,
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
type TypePattern<'a> = Scheme<'a>;
type ConstructorName = Name;
type ApplyExpr<'a, 'src> = &'a [Expr<'a, 'src>; 2];
type ApplyPolyPattern<'a> = &'a [PolyPattern<'a>; 2];
type ApplyMonoPattern<'a> = &'a [MonoPattern<'a>; 2];
type GenScope = ();

pub type ItemNode<'a, 'src> =
    nodes::ItemNode<Expr<'a, 'src>, PolyPattern<'a>, TypePattern<'a>, Data<'a>, GenScope>;

pub type ExprNode<'a, 'src> = nodes::ExprNode<
    'a,
    'src,
    Expr<'a, 'src>,
    PolyPattern<'a>,
    TypeSyntax,
    Name,
    ApplyExpr<'a, 'src>,
    GenScope,
>;

pub type PolyPatternNode<'a> = nodes::PatternNode<
    'a,
    PolyPattern<'a>,
    TypeSyntax,
    Name,
    ConstructorName,
    ApplyPolyPattern<'a>,
>;

pub type MonoPatternNode<'a> = nodes::PatternNode<
    'a,
    MonoPattern<'a>,
    TypeSyntax,
    Name,
    ConstructorName,
    ApplyMonoPattern<'a>,
>;

pub type DataNode<'a> = nodes::DataNode<'a, Constructor<'a>>;

pub type ConstructorNode<'a> = nodes::ConstructorNode<'a, Name, Type<'a>>;

pub(crate) struct BoundItem<'a, E> {
    pub node: BoundItemNode<'a, E>,
    pub span: Span,
    pub id: ItemId,
}

type BoundGenScope<'a> = &'a [Generic];

pub(crate) type BoundItemNode<'a, E> =
    nodes::ItemNode<E, MonoPattern<'a>, TypePattern<'a>, Data<'a>, BoundGenScope<'a>>;
