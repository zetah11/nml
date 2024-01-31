use std::convert::Infallible;

use crate::frontend::errors::{ErrorId, Errors};
use crate::frontend::names::Ident;
use crate::frontend::source::{SourceId, Span};

use super::nodes;

pub struct Source<'a, 'src> {
    pub items: &'a [Item<'a, 'src>],
    pub errors: Errors,
    pub unattached: Vec<(ErrorId, Span)>,
    pub source: SourceId,
}

pub struct Item<'a, 'src> {
    pub node: ItemNode<'a, 'src>,
    pub span: Span,
}

pub struct Expr<'a, 'src> {
    pub node: ExprNode<'a, 'src>,
    pub span: Span,
}

pub struct Pattern<'a, 'src> {
    pub node: PatternNode<'a, 'src>,
    pub span: Span,
}

#[derive(Clone, Copy)]
pub struct Type<'a, 'src> {
    pub node: TypeNode<'a, 'src>,
    pub span: Span,
}

/// The body of a `data` item is a list of [`Constructor`]s.
pub struct Data<'a, 'src> {
    pub node: DataNode<'a, 'src>,
    pub span: Span,
}

/// Every constructor is an (optional) affix, an identifier, and an optional
/// list of types.
pub struct Constructor<'a, 'src> {
    pub node: ConstructorNode<'a, 'src>,
    pub span: Span,
}

type GenScope = ();
type Name<'src> = Ident<'src>;
type PatternVar<'src> = (Affix, Ident<'src>);
type ConstructorName = Infallible;
type Universal<'src> = Ident<'src>;

type ApplyExpr<'a, 'src> = &'a [Expr<'a, 'src>];
type ApplyPattern<'a, 'src> = &'a [Pattern<'a, 'src>];
type ApplyType<'a, 'src> = &'a [Type<'a, 'src>];

pub type ItemNode<'a, 'src> =
    nodes::ItemNode<Expr<'a, 'src>, Pattern<'a, 'src>, Pattern<'a, 'src>, Data<'a, 'src>, GenScope>;

pub type ExprNode<'a, 'src> = nodes::ExprNode<
    'a,
    'src,
    Expr<'a, 'src>,
    Pattern<'a, 'src>,
    Type<'a, 'src>,
    Name<'src>,
    ApplyExpr<'a, 'src>,
    GenScope,
>;

pub type PatternNode<'a, 'src> = nodes::PatternNode<
    'a,
    Pattern<'a, 'src>,
    Type<'a, 'src>,
    PatternVar<'src>,
    ConstructorName,
    ApplyPattern<'a, 'src>,
>;

pub type TypeNode<'a, 'src> =
    nodes::TypeNode<'a, 'src, Type<'a, 'src>, Name<'src>, Universal<'src>, ApplyType<'a, 'src>>;

pub type DataNode<'a, 'src> = nodes::DataNode<'a, Constructor<'a, 'src>>;

pub type ConstructorNode<'a, 'src> =
    nodes::ConstructorNode<'a, (Affix, Name<'src>), Type<'a, 'src>>;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Affix {
    Prefix,
    Infix,
    Postfix,
}
