use std::collections::BTreeMap;
use std::convert::Infallible;

use crate::errors::{ErrorId, Errors};
use crate::names::{Ident, Name};
use crate::resolve::ItemId;
use crate::source::{SourceId, Span};

use super::{nodes, parsed, resolved};

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
    type Expr = &'a parsed::Expr<'a>;
    type Pattern = resolved::Pattern<'a>;

    type ExprName = Infallible;
    type PatternName = Infallible;
    type Var = Infallible;
    type Variant = Infallible;
}

pub struct Item<'a> {
    pub node: ItemNode<'a>,
    pub span: Span,
    pub id: ItemId,
}

pub type ItemNode<'a> = nodes::ItemNode<Data<'a>>;
