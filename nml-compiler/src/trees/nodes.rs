use malachite::Integer;

use crate::errors::ErrorId;
use crate::names::Label;
use crate::source::Span;

pub trait Data {
    type Item;
    type Expr;
    type Pattern;

    type ExprName;
    type PatternName;
    type Var;
    type Variant;

    type Apply;
}

pub enum ItemNode<D: Data> {
    /// Something fishy
    Invalid(ErrorId),

    /// `let a = x`
    Let(D::Pattern, D::Expr),
}

pub enum ExprNode<'a, 'lit, D: Data> {
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
    Number(&'lit Integer),

    /// `if x then y else z`
    If(&'a [D::Expr; 3]),

    /* Records -------------------------------------------------------------- */
    /// `x.a`
    Field(&'a D::Expr, Result<Label, ErrorId>, Span),

    /// `{ a = x, b = y | r }`
    Record(
        &'a [(Result<Label, ErrorId>, Span, D::Expr)],
        Option<&'a D::Expr>,
    ),

    /// `x \ a`
    Restrict(&'a D::Expr, Label),

    /* Variants ------------------------------------------------------------- */
    /// `A`
    Variant(D::Variant),

    /* Functions ------------------------------------------------------------ */
    /// `x y`
    Apply(D::Apply),

    /// `a => x | b => y`
    Lambda(&'a [(D::Pattern, D::Expr)]),

    /// `let a = x in y`
    Let(D::Pattern, &'a [D::Expr; 2]),
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
    Apply(&'a [D::Pattern; 2]),
}

/* Copy and Clone impls ----------------------------------------------------- */

impl<D: Data> Copy for ItemNode<D>
where
    D::Pattern: Copy,
    D::Expr: Copy,
{
}

impl<D: Data> Clone for ItemNode<D>
where
    D::Pattern: Copy,
    D::Expr: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Data> Copy for ExprNode<'_, '_, D>
where
    D::ExprName: Copy,
    D::Var: Copy,
    D::Variant: Copy,
    D::Apply: Copy,
    D::Pattern: Copy,
{
}

impl<D: Data> Clone for ExprNode<'_, '_, D>
where
    D::ExprName: Copy,
    D::Var: Copy,
    D::Variant: Copy,
    D::Apply: Copy,
    D::Pattern: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Data> Copy for PatternNode<'_, D>
where
    D::PatternName: Copy,
    D::Var: Copy,
    D::Variant: Copy,
{
}

impl<D: Data> Clone for PatternNode<'_, D>
where
    D::PatternName: Copy,
    D::Var: Copy,
    D::Variant: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}
