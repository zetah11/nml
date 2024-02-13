#![expect(unused)]

mod checks;

use std::sync::Arc;

use smol_str::SmolStr;

/// A green node is a lossless and immutable syntax tree facilitating sharing.
/// Each node stores its total width in bytes.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Node {
    /// The total width of this node. This must be the equal to
    /// `self.data.width()`.
    pub(super) width: usize,
    pub(super) kind: Kind,
    pub(super) data: Data,
}

/// A node contains either a string (if it is a token) or a set of children.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Data {
    Node(Arc<[Node]>),
    Token(SmolStr),
}

impl Kind {
    /// Returns `true` if this represents something without semantic
    /// significance (other than as a token separator)
    pub fn is_skipped(&self) -> bool {
        matches!(self, Kind::Whitespace | Kind::Comment)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Kind {
    Invalid,

    // Tokens
    /// Arbitrary whitespace
    Whitespace,

    /// A line comment
    Comment,

    /// An identifier
    Name,

    /// An identifier with an initial tick
    PreTick,

    /// An identifier with a trailing tick
    PostTick,

    /// A numeric literal
    Number,

    /// `and`
    And,

    /// `case`
    Case,

    /// `data`
    Data,

    /// `end`
    End,

    /// `in`
    In,

    /// `infix`
    Infix,

    /// `let`
    Let,

    /// `postfix`
    Postfix,

    /// `&`
    Ampersand,

    /// `,`
    Comma,

    /// `.`
    Dot,

    /// `...`
    Ellipses,

    /// `:`
    Colon,

    /// `=`
    Equal,

    /// `=>`
    EqualArrow,

    /// `|`
    Pipe,

    /// `_`
    Underscore,

    /// `(`
    LeftParen,

    /// `)`
    RightParen,

    /// `{`
    LeftBrace,

    /// `}`
    RightBrace,

    // Trees
    /// A source file
    Source,

    /// A tree surrounded by parentheses
    ParenGroup,

    /// A tree surrounded by braces
    BraceGroup,

    /// A tree surrounded by `case` and `end`
    CaseGroup,

    /// Two trees separated by `in`
    Scoped,

    /// A group of `and`-separated trees
    DefinitionGroup,

    /// A single definition
    Definition,

    /// A group of `|`-separated trees
    Disjoined,

    /// A group of `&`-separated trees
    Conjoined,

    /// A group of `=>`-separated trees
    Implied,

    /// A function application
    Apply,

    /// A tree and annotation separated by `:`
    Annotate,

    /// A group of `.`-separated trees
    Qualified,
}
