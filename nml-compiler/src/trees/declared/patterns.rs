use crate::names::Name;
use crate::resolve::ItemId;
use crate::source::Span;

use super::{nodes, parsed};

pub(crate) struct Item<'parsed, 'lit> {
    pub node: ItemNode<'parsed, 'lit>,
    pub span: Span,
    pub id: ItemId,
}

pub(crate) struct Data<'parsed, 'lit> {
    pub node: DataNode<'parsed, 'lit>,
    pub span: Span,
}

pub(crate) struct Constructor<'parsed, 'lit> {
    pub node: ConstructorNode<'parsed, 'lit>,
    pub span: Span,
}

pub(crate) type ItemNode<'parsed, 'lit> = nodes::ItemNode<
    Expr<'parsed, 'lit>,
    Pattern<'parsed, 'lit>,
    TypePattern<'parsed, 'lit>,
    Data<'parsed, 'lit>,
    GenScope,
>;

pub(crate) type DataNode<'parsed, 'lit> = nodes::DataNode<'parsed, Constructor<'parsed, 'lit>>;

pub(crate) type ConstructorNode<'parsed, 'lit> =
    nodes::ConstructorNode<'parsed, Name, parsed::Type<'parsed, 'lit>>;

type Expr<'parsed, 'lit> = &'parsed parsed::Expr<'parsed, 'lit>;
type Pattern<'parsed, 'lit> = &'parsed parsed::Pattern<'parsed, 'lit>;
type TypePattern<'parsed, 'lit> = &'parsed parsed::Pattern<'parsed, 'lit>;
type GenScope = ();
