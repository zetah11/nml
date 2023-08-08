use std::collections::BTreeMap;
use std::convert::Infallible;

use crate::errors::{ErrorId, Errors};
use crate::names::{Ident, Name};
use crate::resolve::ItemId;
use crate::source::{SourceId, Span};

use super::{nodes, parsed};

pub struct Source<'a> {
    pub items: &'a [Item<'a>],
    pub errors: Errors,
    pub unattached: Vec<(ErrorId, Span)>,
    pub source: SourceId,

    pub names: BTreeMap<Ident, Name>,
    pub defines: BTreeMap<Name, (Span, ItemId)>,
}

pub struct Data<'a>(std::marker::PhantomData<&'a ()>);

impl<'a> nodes::Data for Data<'a> {
    type Item = Item<'a>;
    type Expr = parsed::Expr<'a>;
    type Pattern = parsed::Pattern<'a>;

    type ItemName = Name;
    type ExprName = Infallible;
    type PatternName = Infallible;
    type ItemLet = ();
    type LetName = Infallible;
    type LetExtra = Infallible;
    type Var = Infallible;
    type Variant = Infallible;
}

pub struct Item<'a> {
    pub id: ItemId,
    pub node: ItemNode<'a>,
    pub span: Span,
}

pub type ItemNode<'a> = nodes::ItemNode<Data<'a>>;

pub type Expr<'a> = parsed::Expr<'a>;
pub type ExprNode<'a> = parsed::ExprNode<'a>;
pub type Pattern<'a> = parsed::Pattern<'a>;
pub type PatternNode<'a> = parsed::PatternNode<'a>;
