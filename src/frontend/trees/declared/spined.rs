use super::{nodes, parsed};
use crate::frontend::names::{Ident, Name};
use crate::frontend::resolve::ItemId;
use crate::frontend::source::Span;

type Type<'scratch, 'src> = &'scratch parsed::Type<'scratch, 'src>;
type Var<'src> = (parsed::Affix, Ident<'src>);
type ConstructorName = Name;
type ApplyPattern<'scratch, 'src> = &'scratch [Pattern<'scratch, 'src>; 2];

pub(crate) struct Pattern<'scratch, 'src> {
    pub node: PatternNode<'scratch, 'src>,
    pub span: Span,
    pub item_id: ItemId,
}

pub(crate) type PatternNode<'scratch, 'src> = nodes::PatternNode<
    'scratch,
    Pattern<'scratch, 'src>,
    Type<'scratch, 'src>,
    Var<'src>,
    ConstructorName,
    ApplyPattern<'scratch, 'src>,
>;

impl Pattern<'_, '_> {
    pub fn is_constructor(&self) -> bool {
        match &self.node {
            PatternNode::Invalid(_) | PatternNode::Constructor(_) => true,
            PatternNode::Group(pattern) => pattern.is_constructor(),

            _ => false,
        }
    }
}
