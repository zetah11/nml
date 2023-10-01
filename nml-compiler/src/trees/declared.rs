use std::collections::BTreeMap;
use std::convert::Infallible;

use super::{nodes, parsed, resolved};
use crate::names::{Ident, Name};
use crate::resolve::ItemId;
use crate::source::Span;

/// Name resolution is a two-pass process which first _declares_ the various
/// names in scope, and then resolves them. This allows referring to names
/// before their lexical definition.
///
/// A declared syntax tree represents the output of the first pass. Items here
/// have a [`resolved`] pattern (since defining a name also resolves it), but a
/// [`parsed`] expression body (since that part doesn't bring any more names
/// into scope).
///
/// - `'a` - the lifetime of the declared bits
/// - `'b` - the lifetime of the parsed bits
/// - `'lit` - the lifetime of any literals, which must outlive all others
/// - `'sub` - a lifetime outlived by both `'a` and `'b`
pub struct Data<'a, 'b, 'lit, 'sub>(
    std::marker::PhantomData<(&'sub &'a &'lit (), &'sub &'b &'lit ())>,
);

impl<'a, 'b, 'lit, 'sub> nodes::Data<'sub> for Data<'a, 'b, 'lit, 'sub>
where
    'a: 'sub,
    'b: 'sub,
{
    type Item = Item<'a, 'b, 'lit, 'sub>;
    type Expr = &'b parsed::Expr<'b, 'lit>;
    type Pattern = Spine<'a, 'lit>;
    type Type = &'b parsed::Type<'b, 'lit>;

    type ExprName = Infallible;
    type PatternName = Infallible;
    type Var = Infallible;
    type Universal = Infallible;

    type Apply<T: 'sub> = Infallible;
    type GenScope = BTreeMap<Ident<'lit>, Name>;
}

pub struct Item<'a, 'b, 'lit, 'sub>
where
    'a: 'sub,
    'b: 'sub,
{
    pub node: ItemNode<'a, 'b, 'lit, 'sub>,
    pub span: Span,
    pub id: ItemId,
}

pub type ItemNode<'a, 'b, 'lit, 'sub> = nodes::ItemNode<'sub, Data<'a, 'b, 'lit, 'sub>>;

pub enum Spine<'a, 'lit> {
    Fun {
        head: resolved::Pattern<'a, 'lit>,
        args: Vec<resolved::Pattern<'a, 'lit>>,
    },

    Single(resolved::Pattern<'a, 'lit>),
}
