use std::collections::BTreeMap;

use super::nodes;
use crate::frontend::errors::{ErrorId, Errors};
use crate::frontend::names::Name;
use crate::frontend::resolve::ItemId;
use crate::frontend::source::Span;

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
}

pub struct Pattern<'a, 'src> {
    pub node: PatternNode<'a, 'src>,
    pub span: Span,
}

pub struct Type<'a, 'src> {
    pub node: TypeNode<'a, 'src>,
    pub span: Span,
}

pub struct DataPattern<'a> {
    pub name: Result<Name, ErrorId>,
    pub args: &'a [Result<Name, ErrorId>],
}

pub struct Data<'a, 'src> {
    pub node: DataNode<'a, 'src>,
    pub span: Span,
}

pub struct Constructor<'a, 'src> {
    pub node: ConstructorNode<'a, 'src>,
    pub span: Span,
}

type ConstructorName = Name;
type Universal = Name;
type ApplyExpr<'a, 'src> = &'a [Expr<'a, 'src>; 2];
type ApplyPattern<'a, 'src> = &'a [Pattern<'a, 'src>; 2];
type ApplyType<'a, 'src> = &'a [Type<'a, 'src>; 2];
type GenScope<'a> = &'a [Name];

pub type ItemNode<'a, 'src> = nodes::ItemNode<
    Expr<'a, 'src>,
    Pattern<'a, 'src>,
    DataPattern<'a>,
    Data<'a, 'src>,
    GenScope<'a>,
>;

pub type ExprNode<'a, 'src> = nodes::ExprNode<
    'a,
    'src,
    Expr<'a, 'src>,
    Pattern<'a, 'src>,
    Type<'a, 'src>,
    Name,
    ApplyExpr<'a, 'src>,
    GenScope<'a>,
>;

pub type PatternNode<'a, 'src> = nodes::PatternNode<
    'a,
    Pattern<'a, 'src>,
    Type<'a, 'src>,
    Name,
    ConstructorName,
    ApplyPattern<'a, 'src>,
>;

pub type TypeNode<'a, 'src> =
    nodes::TypeNode<'a, 'src, Type<'a, 'src>, Name, Universal, ApplyType<'a, 'src>>;

pub type DataNode<'a, 'src> = nodes::DataNode<'a, Constructor<'a, 'src>>;

pub type ConstructorNode<'a, 'src> = nodes::ConstructorNode<'a, Name, Type<'a, 'src>>;
