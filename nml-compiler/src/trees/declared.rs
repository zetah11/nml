//! Name resolution is a three-pass process:
//! 1. Declare all constructors in scope
//! 2. Declare all remaining names in scope
//! 3. Resolve all names.
//!
//! This three-step process allows for lots of flexibility in referring to names
//! "before" they are (lexically) defined, creating custom infix and postfix
//! functions and constructors, and so on.
//!
//! A declared syntax tree represents the output of the second pass. Items here
//! have a [`resolved`] pattern (since defining a name also resolves it), but a
//! [`parsed`] expression body (since that part doesn't bring any more names
//! into scope).
//!
//! # Spined patterns
//!
//! In order to correctly resolve a pattern, we must first turn it into a spine,
//! which requires figuring out which names are constructors (and their fixity).
//! This affects scoping: consider the following `let`-item:
//!
//! ```nml
//! let f x = ...
//! ```
//!
//! If `f` is a constructor, then the above is a single, destructuring pattern
//! which brings the name `x` into the scope of the `let` item. However, if `f`
//! is not a constructor, then the name `f` is brought into that scope while `x`
//! is bound in the narrower scope of its body.
//!
//! Figuring this out happens after step 1 (declaring constructors) but before
//! step 2, and produces a [`SpinedPattern`]. This is a "partially resolved"
//! pattern, where constructors and application order is explicit, but no names
//! have been defined. A spined pattern can then be defined and resolved with
//! the appropriate scopes.
//!
//! # Lifetime conventions
//!
//! - `'lit` - used for literals and raw identifiers
//! - `'a` - references to [`resolved`] patterns
//! - `'parsed` - references into the [`parsed`] syntax tree
//!
//! [`SpinedPattern`]s only take two lifetimes `'scratch` and `'lit`; they
//! should be temporary and not outlive any [`parsed`] subtrees.

use std::collections::BTreeMap;

use super::{nodes, parsed, resolved};
use crate::names::{Ident, Name};
use crate::resolve::ItemId;
use crate::source::Span;

type Expr<'parsed, 'lit> = &'parsed parsed::Expr<'parsed, 'lit>;
type Pattern<'a, 'parsed, 'lit> = Spine<'parsed, 'lit, resolved::Pattern<'a, 'lit>>;
type GenScope<'lit> = BTreeMap<Ident<'lit>, Name>;

pub(crate) struct Item<'a, 'parsed, 'lit> {
    pub node: ItemNode<'a, 'parsed, 'lit>,
    pub span: Span,
    pub id: ItemId,
}

pub(crate) type ItemNode<'a, 'parsed, 'lit> =
    nodes::ItemNode<Expr<'parsed, 'lit>, Pattern<'a, 'parsed, 'lit>, GenScope<'lit>>;

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

pub(crate) use spined::{Pattern as SpinedPattern, PatternNode as SpinedPatternNode};

mod spined {
    use super::{nodes, parsed};
    use crate::names::{Ident, Name};
    use crate::resolve::ItemId;
    use crate::source::Span;

    type Type<'scratch, 'lit> = &'scratch parsed::Type<'scratch, 'lit>;
    type Var<'lit> = (parsed::Affix, Ident<'lit>);
    type Constructor = Name;
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
        Constructor,
        ApplyPattern<'scratch, 'lit>,
    >;
}
