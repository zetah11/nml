use crate::frontend::names::Name;
use crate::frontend::resolve::ItemId;
use crate::frontend::source::Span;

use super::{nodes, parsed};

pub(crate) struct Item<'parsed, 'src> {
    pub node: ItemNode<'parsed, 'src>,
    pub span: Span,
    pub id: ItemId,
}

pub(crate) struct Data<'parsed, 'src> {
    pub node: DataNode<'parsed, 'src>,
    pub span: Span,
}

pub(crate) struct Constructor<'parsed, 'src> {
    pub node: ConstructorNode<'parsed, 'src>,
    pub span: Span,
}

pub(crate) type ItemNode<'parsed, 'src> = nodes::ItemNode<
    Expr<'parsed, 'src>,
    Pattern<'parsed, 'src>,
    TypePattern<'parsed, 'src>,
    Data<'parsed, 'src>,
    GenScope,
>;

pub(crate) type DataNode<'parsed, 'src> = nodes::DataNode<'parsed, Constructor<'parsed, 'src>>;

pub(crate) type ConstructorNode<'parsed, 'src> =
    nodes::ConstructorNode<'parsed, Name, parsed::Type<'parsed, 'src>>;

type Expr<'parsed, 'src> = &'parsed parsed::Expr<'parsed, 'src>;
type Pattern<'parsed, 'src> = &'parsed parsed::Pattern<'parsed, 'src>;
type TypePattern<'parsed, 'src> = &'parsed parsed::Pattern<'parsed, 'src>;
type GenScope = ();
