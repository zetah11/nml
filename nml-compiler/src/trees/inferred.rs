use std::convert::Infallible;

use crate::names::{Label, Name};
use crate::resolve::ItemId;
use crate::source::Span;
use crate::tyck::{Scheme, Type};

use super::nodes;

pub struct Data<'a>(std::marker::PhantomData<&'a ()>);

pub struct MonoData<'a>(std::marker::PhantomData<&'a ()>);

impl<'a> nodes::Data for Data<'a> {
    type Item = Item<'a>;
    type Expr = Expr<'a>;
    type Pattern = PolyPattern<'a>;

    type ItemName = Name;
    type ExprName = Infallible;
    type PatternName = Infallible;
    type ItemLet = ();
    type LetName = Name;
    type LetExtra = ();
    type Var = Name;
    type Variant = Label;
}

impl<'a> nodes::Data for MonoData<'a> {
    type Item = Infallible;
    type Expr = Infallible;
    type Pattern = MonoPattern<'a>;

    type ItemName = Infallible;
    type ExprName = Infallible;
    type PatternName = Infallible;
    type ItemLet = Infallible;
    type LetName = Infallible;
    type LetExtra = Infallible;
    type Var = Name;
    type Variant = Label;
}

pub struct Item<'a> {
    pub node: ItemNode<'a>,
    pub span: Span,
    pub id: ItemId,
}

pub struct Expr<'a> {
    pub node: ExprNode<'a>,
    pub span: Span,
    pub ty: &'a Type<'a>,
}

pub struct PolyPattern<'a> {
    pub node: PolyPatternNode<'a>,
    pub span: Span,
    pub scheme: Scheme<'a>,
}

pub struct MonoPattern<'a> {
    pub node: MonoPatternNode<'a>,
    pub span: Span,
    pub ty: &'a Type<'a>,
}

pub type ItemNode<'a> = nodes::ItemNode<Data<'a>>;
pub type ExprNode<'a> = nodes::ExprNode<'a, Data<'a>>;
pub type PolyPatternNode<'a> = nodes::PatternNode<'a, Data<'a>>;
pub type MonoPatternNode<'a> = nodes::PatternNode<'a, MonoData<'a>>;
