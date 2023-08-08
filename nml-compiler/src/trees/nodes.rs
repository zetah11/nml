use malachite::Integer;

use crate::errors::ErrorId;
use crate::names::Label;
use crate::source::Span;

pub trait Data {
    type Item;
    type Expr;
    type Pattern;

    type ItemName;
    type ExprName;
    type PatternName;
    type ItemLet;
    type LetName;
    type LetExtra;
    type Var;
    type Variant;
}

pub enum ItemNode<D: Data> {
    /// Something fishy
    Invalid(ErrorId),

    /// `let a = x`
    Let(Result<D::ItemName, ErrorId>, D::ItemLet, D::Expr),
}

pub enum ExprNode<'a, D: Data> {
    /// Something fishy
    Invalid(ErrorId),

    /// Variable name
    Var(D::Var),

    /// A name with a lowercase initial.
    Small(D::ExprName),

    /// A name with an uppercase initial.
    Big(D::ExprName),

    /// `_`
    Hole,

    /// `()`
    Unit,

    /// `true` or `false`
    Bool(bool),

    /// Some integer
    Number(Integer),

    /// `if x then y else z`
    If(&'a D::Expr, &'a D::Expr, &'a D::Expr),

    /* Records -------------------------------------------------------------- */
    /// `x.a`
    Field(&'a D::Expr, Result<Label, ErrorId>, Span),

    /// `{ a = x, b = y | r }`
    Record(&'a [(Result<Label, ErrorId>, Span, D::Expr)], Option<&'a D::Expr>),

    /// `x \ a`
    Restrict(&'a D::Expr, Label),

    /* Variants ------------------------------------------------------------- */
    /// `A`
    Variant(D::Variant),

    /// `case x | p => y | q => z end`
    Case { scrutinee: &'a D::Expr, cases: &'a [(D::Pattern, D::Expr)] },

    /* Functions ------------------------------------------------------------ */
    /// `x y`
    Apply(&'a D::Expr, &'a D::Expr),

    /// `a => x`
    Lambda(D::Pattern, &'a D::Expr),

    /// `let a = x in y`
    Let(D::Pattern, &'a D::Expr, &'a D::Expr),
}

pub enum PatternNode<'a, D: Data> {
    /// Something fishy.
    Invalid(ErrorId),

    /// `_`
    Wildcard,

    /// `()`
    Unit,

    /// A name with a lowercase initial.
    Small(D::PatternName),

    /// A name with an uppercase initial.
    Big(D::PatternName),

    /// A name binding
    Bind(D::Var),

    /// A named pattern (e.g. a defined constructor)
    Named(D::Var),

    /// An anonymous variant
    Deconstruct(D::Variant, &'a D::Pattern),

    /// A pattern application
    Apply(&'a D::Pattern, &'a D::Pattern),
}