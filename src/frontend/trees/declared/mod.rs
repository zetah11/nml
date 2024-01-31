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
//! step 2, and produces a [`spined::Pattern`]. This is a "partially resolved"
//! pattern, where constructors and application order is explicit, but no names
//! have been defined. A spined pattern can then be defined and resolved with
//! the appropriate scopes.
//!
//! # Lifetime conventions
//!
//! - `'src` - used for literals and raw identifiers
//! - `'a` - references to [`resolved`] patterns
//! - `'parsed` - references into the [`parsed`] syntax tree
//!
//! [`spined::Pattern`]s only take two lifetimes `'scratch` and `'src`; they
//! should be temporary and not outlive any [`parsed`] subtrees.

pub(crate) mod patterns;
pub(crate) mod spined;

use std::collections::BTreeMap;

use super::{nodes, parsed, resolved};
use crate::frontend::names::{Ident, Name};
use crate::frontend::resolve::ItemId;
use crate::frontend::source::Span;

pub(crate) struct Item<'a, 'parsed, 'src> {
    pub node: ItemNode<'a, 'parsed, 'src>,
    pub span: Span,
    pub id: ItemId,
}

type Expr<'parsed, 'src> = &'parsed parsed::Expr<'parsed, 'src>;
type Pattern<'a, 'parsed, 'src> = Spine<'parsed, 'src, resolved::Pattern<'a, 'src>>;
type TypePattern<'a, 'parsed, 'src> = Spine<'parsed, 'src, resolved::Pattern<'a, 'src>>;
type Data<'parsed, 'src> = patterns::Data<'parsed, 'src>;
type GenScope<'src> = BTreeMap<Ident<'src>, Name>;

pub(crate) type ItemNode<'a, 'parsed, 'src> = nodes::ItemNode<
    Expr<'parsed, 'src>,
    Pattern<'a, 'parsed, 'src>,
    TypePattern<'a, 'parsed, 'src>,
    Data<'parsed, 'src>,
    GenScope<'src>,
>;

pub(crate) enum Spine<'a, 'src, T> {
    Fun {
        head: T,
        args: Vec<spined::Pattern<'a, 'src>>,
        anno: Option<&'a parsed::Type<'a, 'src>>,
    },

    Single(T),
}

impl<'a, 'src, T> Spine<'a, 'src, T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spine<'a, 'src, U> {
        match self {
            Self::Fun { head, args, anno } => Spine::Fun {
                head: f(head),
                args,
                anno,
            },

            Self::Single(pattern) => Spine::Single(f(pattern)),
        }
    }
}
