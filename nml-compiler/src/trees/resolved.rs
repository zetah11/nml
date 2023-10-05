use std::collections::BTreeMap;
use std::convert::Infallible;

use super::nodes;
use crate::errors::{ErrorId, Errors};
use crate::names::Name;
use crate::resolve::ItemId;
use crate::source::Span;

pub struct Program<'a, 'lit> {
    pub items: &'a [&'a [Item<'a, 'lit>]],
    pub defs: BTreeMap<Name, Span>,
    pub errors: Errors,
    pub unattached: Vec<(ErrorId, Span)>,
}

pub struct Data<'a, 'lit>(std::marker::PhantomData<&'a &'lit ()>);

pub struct TypeData<'a, 'lit>(std::marker::PhantomData<&'a &'lit ()>);

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

pub struct Type<'a, 'lit> {
    pub node: TypeNode<'a, 'lit>,
    pub span: Span,
}

type GenScope<'a> = &'a [Name];
type Var = Name;
type Universal = Name;
type ApplyExpr<'a, 'lit> = &'a [Expr<'a, 'lit>; 2];
type ApplyPattern<'a, 'lit> = &'a [Pattern<'a, 'lit>; 2];

pub type ItemNode<'a, 'lit> = nodes::ItemNode<Expr<'a, 'lit>, Pattern<'a, 'lit>, GenScope<'a>>;

pub type ExprNode<'a, 'lit> = nodes::ExprNode<
    'a,
    'lit,
    Expr<'a, 'lit>,
    Pattern<'a, 'lit>,
    Type<'a, 'lit>,
    Infallible,
    Var,
    ApplyExpr<'a, 'lit>,
    GenScope<'a>,
>;

pub type PatternNode<'a, 'lit> = nodes::PatternNode<
    'a,
    Pattern<'a, 'lit>,
    Type<'a, 'lit>,
    Infallible,
    Var,
    ApplyPattern<'a, 'lit>,
>;

pub type TypeNode<'a, 'lit> = nodes::TypeNode<'a, 'lit, Type<'a, 'lit>, Universal>;
