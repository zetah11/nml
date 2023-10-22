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

#[derive(Clone, Copy)]
pub struct Type<'a, 'lit> {
    pub node: TypeNode<'a, 'lit>,
    pub span: Span,
}

/// The body of a `data` item is a list of [`Constructor`]s.
pub struct Data<'a, 'lit> {
    pub node: DataNode<'a, 'lit>,
    pub span: Span,
}

/// Every constructor is an (optional) affix, an identifier, and an optional
/// list of types.
pub struct Constructor<'a, 'lit> {
    pub node: ConstructorNode<'a, 'lit>,
    pub span: Span,
}

type GenScope = ();
type Name<'lit> = Ident<'lit>;
type PatternVar<'lit> = (Affix, Ident<'lit>);
type ConstructorName = Infallible;
type Universal<'lit> = Ident<'lit>;

type ApplyExpr<'a, 'lit> = &'a [Expr<'a, 'lit>];
type ApplyPattern<'a, 'lit> = &'a [Pattern<'a, 'lit>];

pub type ItemNode<'a, 'lit> =
    nodes::ItemNode<Expr<'a, 'lit>, Pattern<'a, 'lit>, Pattern<'a, 'lit>, Data<'a, 'lit>, GenScope>;

pub type ExprNode<'a, 'lit> = nodes::ExprNode<
    'a,
    'lit,
    Expr<'a, 'lit>,
    Pattern<'a, 'lit>,
    Type<'a, 'lit>,
    Name<'lit>,
    ApplyExpr<'a, 'lit>,
    GenScope,
>;

pub type PatternNode<'a, 'lit> = nodes::PatternNode<
    'a,
    Pattern<'a, 'lit>,
    Type<'a, 'lit>,
    PatternVar<'lit>,
    ConstructorName,
    ApplyPattern<'a, 'lit>,
>;

pub type TypeNode<'a, 'lit> =
    nodes::TypeNode<'a, 'lit, Type<'a, 'lit>, Name<'lit>, Universal<'lit>>;

pub type DataNode<'a, 'lit> = nodes::DataNode<'a, Constructor<'a, 'lit>>;

pub type ConstructorNode<'a, 'lit> =
    nodes::ConstructorNode<'a, (Affix, Name<'lit>), Type<'a, 'lit>>;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Affix {
    Prefix,
    Infix,
    Postfix,
}
