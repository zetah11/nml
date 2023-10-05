use std::collections::BTreeMap;

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
pub(crate) struct Data<'a, 'b, 'lit>(std::marker::PhantomData<(&'a &'lit (), &'b &'lit ())>);

type GenScope<'lit> = BTreeMap<Ident<'lit>, Name>;
pub(crate) type Expr<'b, 'lit> = &'b parsed::Expr<'b, 'lit>;
pub(crate) type Pattern<'a, 'b, 'lit> = Spine<'b, 'lit, resolved::Pattern<'a, 'lit>>;

pub(crate) struct Item<'a, 'b, 'lit> {
    pub node: ItemNode<'a, 'b, 'lit>,
    pub span: Span,
    pub id: ItemId,
}

pub(crate) type ItemNode<'a, 'b, 'lit> =
    nodes::ItemNode<Expr<'b, 'lit>, Pattern<'a, 'b, 'lit>, GenScope<'lit>>;

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

type SpinedPatternName<'lit> = (parsed::Affix, Ident<'lit>);
type SpinedVar = Name;
type SpinedApplyPattern<'a, 'lit> = &'a [SpinedPattern<'a, 'lit>; 2];

pub(crate) type SpinedType<'a, 'lit> = &'a parsed::Type<'a, 'lit>;

pub(crate) struct SpinedPattern<'a, 'lit> {
    pub node: SpinedPatternNode<'a, 'lit>,
    pub span: Span,
    pub item_id: ItemId,
}

pub(crate) type SpinedPatternNode<'a, 'lit> = nodes::PatternNode<
    'a,
    SpinedPattern<'a, 'lit>,
    SpinedType<'a, 'lit>,
    SpinedPatternName<'lit>,
    SpinedVar,
    SpinedApplyPattern<'a, 'lit>,
>;
