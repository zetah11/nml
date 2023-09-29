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

impl<'a, 'lit> nodes::Data for Data<'a, 'lit> {
    type Item = Item<'a, 'lit>;
    type Expr = Expr<'a, 'lit>;
    type Pattern = Pattern<'a, 'lit>;
    type Type = Type<'a, 'lit>;

    type ExprName = Ident<'lit>;
    type PatternName = (Affix, Ident<'lit>);
    type Var = Infallible;
    type Variant = Infallible;
    type Universal = Ident<'lit>;

    type Apply = &'a [Self::Expr];
    type GenScope = ();
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

pub struct Type<'a, 'lit> {
    pub node: TypeNode<'a, 'lit>,
    pub span: Span,
}

pub type ItemNode<'a, 'lit> = nodes::ItemNode<Data<'a, 'lit>>;
pub type ExprNode<'a, 'lit> = nodes::ExprNode<'a, 'lit, Data<'a, 'lit>>;
pub type PatternNode<'a, 'lit> = nodes::PatternNode<'a, Data<'a, 'lit>>;
pub type TypeNode<'a, 'lit> = nodes::TypeNode<'a, 'lit, Data<'a, 'lit>>;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Affix {
    Prefix,
    Infix,
    Postfix,
}
