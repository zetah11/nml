use std::collections::BTreeMap;
use std::convert::Infallible;

use super::{nodes, parsed, resolved};
use crate::names::{Ident, Name};
use crate::resolve::ItemId;
use crate::source::Span;

/// Name resolution is a three-pass process:
/// 1. Declare all patterns in scope
/// 2. Declare all remaining names in scope
/// 3. Resolve all names.
///
/// This three-step process allows for lots of flexibility in referring to names
/// "before" they are (lexically) defined, creating custom infix and postfix
/// functions and constructors, and so on.
///
/// A declared syntax tree represents the output of the second pass. Items here
/// have a [`resolved`] pattern (since defining a name also resolves it), but a
/// [`parsed`] expression body (since that part doesn't bring any more names
/// into scope).
///
/// - `'a` - the lifetime of the declared bits
/// - `'b` - the lifetime of the parsed bits
/// - `'lit` - the lifetime of any literals, which must outlive all others
/// - `'sub` - a lifetime outlived by both `'a` and `'b`
pub(crate) struct Data<'a, 'b, 'lit, 'sub>(
    std::marker::PhantomData<(&'sub &'a &'lit (), &'sub &'b &'lit ())>,
);

impl<'a, 'b, 'lit, 'sub> nodes::Data<'sub> for Data<'a, 'b, 'lit, 'sub>
where
    'a: 'sub,
    'b: 'sub,
{
    type Item = Item<'a, 'b, 'lit, 'sub>;
    type Expr = &'b parsed::Expr<'b, 'lit>;
    type Pattern = Spine<'a, 'lit, resolved::Pattern<'a, 'lit>>;
    type Type = &'b parsed::Type<'b, 'lit>;

    type ExprName = Infallible;
    type PatternName = Infallible;
    type Var = Infallible;
    type Universal = Infallible;

    type Apply<T: 'sub> = Infallible;
    type GenScope = BTreeMap<Ident<'lit>, Name>;
}

pub(crate) struct Item<'a: 'sub, 'b: 'sub, 'lit, 'sub> {
    pub node: ItemNode<'a, 'b, 'lit, 'sub>,
    pub span: Span,
    pub id: ItemId,
}

pub(crate) type ItemNode<'a, 'b, 'lit, 'sub> = nodes::ItemNode<'sub, Data<'a, 'b, 'lit, 'sub>>;

/// In order to correctly resolve a pattern, we must first turn it into a spine,
/// which requires figuring out which names are constructors (and their fixity).
/// This affects scoping: consider the following `let`-item:
///
/// ```nml
/// Let f x = ...
/// ```
///
/// If `f` is a constructor, then the above is a _single deconstructing_ pattern
/// which brings the name `x` into the scope of the `let`. However, if `f` is
/// not a constructor, then the name `f` is brought into that scope while `x` is
/// only visible in the narrower scope of its body.
///
/// A spine is either a single pattern or a function "head" followed by a series
/// of pattern arguments.
///
/// # Lifetimes
///
/// - `'a` - the lifetime of the syntax trees
/// - `'lit` - the lifetime of the literals
pub(crate) struct SpinedData<'a, 'lit>(std::marker::PhantomData<&'a &'lit ()>);

impl<'a, 'lit> nodes::Data<'a> for SpinedData<'a, 'lit> {
    type Item = Infallible;
    type Expr = &'a parsed::Expr<'a, 'lit>;
    type Pattern = SpinedPattern<'a, 'lit>;
    type Type = &'a parsed::Type<'a, 'lit>;

    type ExprName = Infallible;
    type PatternName = (parsed::Affix, Ident<'lit>);
    type Var = Name;
    type Universal = Infallible;

    type Apply<T: 'a> = &'a [T; 2];
    type GenScope = BTreeMap<Ident<'lit>, Name>;
}

pub(crate) enum Spine<'a, 'lit, T> {
    Fun {
        head: T,
        args: Vec<SpinedPattern<'a, 'lit>>,
    },

    Single(T),
}

impl<'a, 'lit, T> Spine<'a, 'lit, T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spine<'a, 'lit, U> {
        match self {
            Self::Fun { head, args } => Spine::Fun {
                head: f(head),
                args,
            },

            Self::Single(pattern) => Spine::Single(f(pattern)),
        }
    }
}

pub(crate) struct SpinedPattern<'a, 'lit> {
    pub node: SpinedPatternNode<'a, 'lit>,
    pub span: Span,
    pub item_id: ItemId,
}

pub(crate) type SpinedPatternNode<'a, 'lit> = nodes::PatternNode<'a, 'a, SpinedData<'a, 'lit>>;
