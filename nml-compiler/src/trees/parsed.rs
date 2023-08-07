use std::convert::Infallible;

use crate::errors::{ErrorId, Errors};
use crate::names::Ident;
use crate::source::{SourceId, Span};

use super::nodes;

pub struct Source<'a> {
    pub items: &'a [Item<'a>],
    pub errors: Errors,
    pub unattached: Vec<(ErrorId, Span)>,
    pub source: SourceId,
}

pub struct Data<'a>(std::marker::PhantomData<&'a ()>);

impl<'a> nodes::Data for Data<'a> {
    type Item = Item<'a>;
    type Expr = Expr<'a>;
    type Pattern = Pattern<'a>;

    type ItemName = Ident;
    type ExprName = Ident;
    type PatternName = Ident;
    type ItemLet = Span;
    type LetName = Ident;
    type LetExtra = Span;
    type Var = Infallible;
    type Variant = Infallible;
}

pub struct Item<'a> {
    pub node: ItemNode<'a>,
    pub span: Span,
}

pub struct Expr<'a> {
    pub node: ExprNode<'a>,
    pub span: Span,
}

pub struct Pattern<'a> {
    pub node: PatternNode<'a>,
    pub span: Span,
}

pub type ItemNode<'a> = nodes::ItemNode<Data<'a>>;
pub type ExprNode<'a> = nodes::ExprNode<'a, Data<'a>>;
pub type PatternNode<'a> = nodes::PatternNode<'a, Data<'a>>;
