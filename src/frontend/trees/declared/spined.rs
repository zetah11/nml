use super::{nodes, parsed};
use crate::frontend::names::{Ident, Name};
use crate::frontend::resolve::ItemId;
use crate::frontend::source::Span;

type Type<'scratch, 'lit> = &'scratch parsed::Type<'scratch, 'lit>;
type Var<'lit> = (parsed::Affix, Ident<'lit>);
type ConstructorName = Name;
type ApplyPattern<'scratch, 'lit> = &'scratch [Pattern<'scratch, 'lit>; 2];

pub(crate) struct Pattern<'scratch, 'lit> {
    pub node: PatternNode<'scratch, 'lit>,
    pub span: Span,
    pub item_id: ItemId,
}

pub(crate) type PatternNode<'scratch, 'lit> = nodes::PatternNode<
    'scratch,
    Pattern<'scratch, 'lit>,
    Type<'scratch, 'lit>,
    Var<'lit>,
    ConstructorName,
    ApplyPattern<'scratch, 'lit>,
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
