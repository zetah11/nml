use malachite::Integer;

use crate::errors::ErrorId;
use crate::names::{Label, Name};
use crate::source::Span;

#[derive(Clone, Debug)]
pub struct Expr<'a> {
    pub node: ExprNode<'a>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprNode<'a> {
    /// Something fishy
    Invalid(ErrorId),

    /// Variable name
    Var(Name),

    /// `true` or `false`
    Bool(bool),

    /// Some integer
    Number(Integer),

    /// `if x then y else z`
    If(&'a Expr<'a>, &'a Expr<'a>, &'a Expr<'a>),

    /* Records -------------------------------------------------------------- */
    /// `x.a`
    Field(&'a Expr<'a>, Label),

    /// `{ a = x, b = y | r }`
    Record(Vec<(Label, &'a Expr<'a>)>, Option<&'a Expr<'a>>),

    /// `x \ a`
    Restrict(&'a Expr<'a>, Label),

    /* Variants ------------------------------------------------------------- */
    /// `A`
    Variant(Label),

    /// `case x | p -> y | q -> z end`
    Case {
        scrutinee: &'a Expr<'a>,
        cases: Vec<(&'a Pattern<'a>, &'a Expr<'a>)>,
    },

    /* Functions ------------------------------------------------------------ */
    /// `x y`
    Apply(&'a Expr<'a>, &'a Expr<'a>),

    /// `a => x`
    Lambda(Name, &'a Expr<'a>),

    /// `let a = x in y`
    Let(Name, &'a Expr<'a>, &'a Expr<'a>),
}

#[derive(Clone, Debug)]
pub struct Pattern<'a> {
    pub node: PatternNode<'a>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum PatternNode<'a> {
    /// Something's not right
    Invalid(ErrorId),

    /// `_`
    Wildcard,

    /// `a`
    Bind(Name),

    /// `M p`
    Deconstruct(Label, &'a Pattern<'a>),
}
