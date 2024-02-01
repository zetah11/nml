use crate::frontend::names::Label;
use crate::frontend::tyck::Type;

use super::solve::TypeVar;

pub enum Message<'a, 'src> {
    /// Emitted when two types fail to unify
    Inequal(&'a Type<'a>, &'a Type<'a>),

    /// Emitted when two row types share a tail but have a distinct prefix (e.g.
    /// `{ x : int | r }` and `{ y : int | r }`)
    DistinctPrefix(Label<'src>, Label<'src>),

    /// Emitted when a row type does not include a label expected of it.
    NoSuchLabel(Label<'src>),

    /// Emitted when a type var is attempted unified with a type containing it.
    /// These kinds of recursive types are disallowed.
    Recursive(TypeVar, &'a Type<'a>),

    /// Emitted when a typed hole is encountered.
    Hole(&'a Type<'a>),
}
