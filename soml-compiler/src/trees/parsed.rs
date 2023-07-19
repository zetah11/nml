use malachite::Integer;

use crate::errors::{ErrorId, Errors};
use crate::names::{Ident, Label};
use crate::source::Span;

#[derive(Debug)]
pub struct Program<'a> {
    pub items: &'a [Item<'a>],
    pub errors: Errors,
    pub unattached: Vec<(ErrorId, Span)>,
}

#[derive(Clone, Debug)]
pub struct Item<'a> {
    pub node: ItemNode<'a>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ItemNode<'a> {
    Invalid(ErrorId),
    Let(Result<Ident, ErrorId>, Span, &'a Expr<'a>),
}

#[derive(Clone, Debug)]
pub struct Expr<'a> {
    pub node: ExprNode<'a>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprNode<'a> {
    Invalid(ErrorId),

    Hole,
    Unit,

    Small(Ident),
    Big(Ident),
    Number(Integer),

    If(&'a Expr<'a>, &'a Expr<'a>, &'a Expr<'a>),

    Field(&'a Expr<'a>, Result<Label, ErrorId>, Span),
    Record(&'a [(Result<Label, ErrorId>, Span, &'a Expr<'a>)], Option<&'a Expr<'a>>),

    Case(&'a Expr<'a>, &'a [(&'a Pattern<'a>, &'a Expr<'a>)]),

    Apply(&'a Expr<'a>, &'a Expr<'a>),
    Lambda(&'a Pattern<'a>, &'a Expr<'a>),
    Let(Result<Ident, ErrorId>, Span, &'a Expr<'a>, &'a Expr<'a>),
}

#[derive(Clone, Debug)]
pub struct Pattern<'a> {
    pub node: PatternNode<'a>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum PatternNode<'a> {
    Invalid(ErrorId),
    Wildcard,
    Small(Ident),
    Big(Ident),
    Apply(&'a Pattern<'a>, &'a Pattern<'a>),
}