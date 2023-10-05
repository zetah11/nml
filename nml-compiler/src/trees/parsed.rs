use std::convert::Infallible;

use crate::errors::{ErrorId, Errors};
use crate::names::Ident;
use crate::source::{SourceId, Span};

use super::nodes;

pub struct Source<'a, 'lit> {
    pub items: &'a [Item<'a, 'lit>],
    pub errors: Errors,
    pub unattached: Vec<(ErrorId, Span)>,
    pub source: SourceId,
}

pub struct Data<'a, 'lit>(std::marker::PhantomData<&'a &'lit ()>);

pub struct Item<'a, 'lit> {
    pub node: ItemNode<'a, 'lit>,
    pub span: Span,
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

type GenScope = ();
type Var<'lit> = Ident<'lit>;
type PatternVar<'lit> = (Affix, Ident<'lit>);
type Constrcutor = Infallible;
type Universal<'lit> = Ident<'lit>;

type ApplyExpr<'a, 'lit> = &'a [Expr<'a, 'lit>];
type ApplyPattern<'a, 'lit> = &'a [Pattern<'a, 'lit>];

pub type ItemNode<'a, 'lit> = nodes::ItemNode<Expr<'a, 'lit>, Pattern<'a, 'lit>, GenScope>;

pub type ExprNode<'a, 'lit> = nodes::ExprNode<
    'a,
    'lit,
    Expr<'a, 'lit>,
    Pattern<'a, 'lit>,
    Type<'a, 'lit>,
    Var<'lit>,
    ApplyExpr<'a, 'lit>,
    GenScope,
>;

pub type PatternNode<'a, 'lit> = nodes::PatternNode<
    'a,
    Pattern<'a, 'lit>,
    Type<'a, 'lit>,
    PatternVar<'lit>,
    Constrcutor,
    ApplyPattern<'a, 'lit>,
>;

pub type TypeNode<'a, 'lit> = nodes::TypeNode<'a, 'lit, Type<'a, 'lit>, Universal<'lit>>;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Affix {
    Prefix,
    Infix,
    Postfix,
}
