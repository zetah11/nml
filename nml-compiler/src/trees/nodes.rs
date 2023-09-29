use malachite::Integer;

use crate::errors::ErrorId;
use crate::names::Label;
use crate::source::Span;

pub trait Data {
    /// An item node in the syntax tree.
    type Item;
    /// An expression node in the syntax tree.
    type Expr;
    /// A pattern node in the syntax tree.
    type Pattern;
    /// A type node in the syntax tree (i.e. a type as it appears in the source,
    /// not an inferred type).
    type Type;

    /// A small or big name within an expression.
    type ExprName;
    /// A small or big name within a pattern.
    type PatternName;
    /// A variable name.
    type Var;
    /// A universal type parameter (`'a`).
    type Universal;

    /// The representation of an application expression.
    type Apply;
    /// Additional data bound at a generalization scope.
    type GenScope;
}

pub enum ItemNode<D: Data> {
    /// Something fishy
    Invalid(ErrorId),

    /// `let a = x`
    Let(D::Pattern, D::Expr, D::GenScope),
}

pub enum ExprNode<'a, 'lit, D: Data> {
    /// Something fishy
    Invalid(ErrorId),

    /// Variable name
    Var(D::Var),

    /// Some name.
    Name(D::ExprName),

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

    /// `x : t`
    Anno(&'a D::Expr, D::Type),

    /// `(x)`
    Group(&'a D::Expr),

    /* Records -------------------------------------------------------------- */
    /// `x.a`
    Field(&'a D::Expr, Result<Label<'lit>, ErrorId>, Span),

    /// `{ a = x, b = y | r }`
    Record(
        &'a [(Result<Label<'lit>, ErrorId>, Span, D::Expr)],
        Option<&'a D::Expr>,
    ),

    /// `x \ a`
    Restrict(&'a D::Expr, Label<'lit>),

    /* Functions ------------------------------------------------------------ */
    /// `x y`
    Apply(D::Apply),

    /// `a => x | b => y`
    Lambda(&'a [(D::Pattern, D::Expr)]),

    /// `let a = x in y`
    Let(D::Pattern, &'a [D::Expr; 2], D::GenScope),
}

pub enum PatternNode<'a, D: Data> {
    /// Something fishy.
    Invalid(ErrorId),

    /// `_`
    Wildcard,

    /// `()`
    Unit,

    /// Some name.
    Name(D::PatternName),

    /// A name binding
    Bind(D::Var),

    /// A named pattern (e.g. a defined constructor)
    Named(D::Var),

    /// `a : t`
    Anno(&'a D::Pattern, D::Type),

    /// A pattern application
    Apply(&'a [D::Pattern; 2]),
}

pub enum TypeNode<'a, 'lit, D: Data> {
    /// Bad stuff.
    Invalid(ErrorId),

    /// `_`
    Wildcard,

    /// `'a`
    Universal(D::Universal),

    /// `t -> u`
    Function(&'a [D::Type; 2]),

    /// `{ a : t, b : u }`
    Record(&'a [(Result<Label<'lit>, ErrorId>, Span, D::Type)]),
}

/* Copy and Clone impls ----------------------------------------------------- */

impl<D: Data> Copy for ItemNode<D>
where
    D::Pattern: Copy,
    D::Expr: Copy,
    D::GenScope: Copy,
{
}

impl<D: Data> Clone for ItemNode<D>
where
    D::Pattern: Copy,
    D::Expr: Copy,
    D::GenScope: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Data> Copy for ExprNode<'_, '_, D>
where
    D::Pattern: Copy,
    D::Type: Copy,
    D::ExprName: Copy,
    D::Var: Copy,
    D::Apply: Copy,
    D::GenScope: Copy,
{
}

impl<D: Data> Clone for ExprNode<'_, '_, D>
where
    D::Pattern: Copy,
    D::Type: Copy,
    D::ExprName: Copy,
    D::Var: Copy,
    D::Apply: Copy,
    D::GenScope: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Data> Copy for PatternNode<'_, D>
where
    D::Type: Copy,
    D::PatternName: Copy,
    D::Var: Copy,
{
}

impl<D: Data> Clone for PatternNode<'_, D>
where
    D::Type: Copy,
    D::PatternName: Copy,
    D::Var: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<D: Data> Copy for TypeNode<'_, '_, D> where D::Universal: Copy {}

impl<D: Data> Clone for TypeNode<'_, '_, D>
where
    D::Universal: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}
