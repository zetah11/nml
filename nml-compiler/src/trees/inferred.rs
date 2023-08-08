use std::convert::Infallible;

use super::nodes;
use crate::names::{Label, Name};
use crate::resolve::ItemId;
use crate::source::Span;
use crate::tyck::{Scheme, Type};

pub struct Data<'a>(std::marker::PhantomData<&'a ()>);

pub struct MonoData<'a>(std::marker::PhantomData<&'a ()>);

impl<'a> nodes::Data for Data<'a> {
    type Item = Item<'a>;
    type Expr = Expr<'a>;
    type Pattern = PolyPattern<'a>;

    type ExprName = Infallible;
    type PatternName = Infallible;
    type Var = Name;
    type Variant = Label;
}

impl<'a> nodes::Data for MonoData<'a> {
    type Item = Infallible;
    type Expr = Infallible;
    type Pattern = MonoPattern<'a>;

    type ExprName = Infallible;
    type PatternName = Infallible;
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

pub(crate) struct BoundItem<'a, T> {
    pub node: BoundItemNode<'a, T>,
    pub span: Span,
    pub id: ItemId,
}

pub(crate) type BoundItemNode<'a, T> = nodes::ItemNode<BoundData<'a, T>>;

pub(crate) struct BoundData<'a, T>(std::marker::PhantomData<&'a T>);

impl<'a, T> nodes::Data for BoundData<'a, T> {
    type Item = BoundItem<'a, T>;
    type Expr = T;
    type Pattern = MonoPattern<'a>;

    type ExprName = Infallible;
    type PatternName = Infallible;
    type Var = Name;
    type Variant = Name;
}
