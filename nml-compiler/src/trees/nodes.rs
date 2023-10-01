use malachite::Integer;

use crate::errors::ErrorId;
use crate::names::Label;
use crate::source::Span;

/// The associated types should all outlive the `'bound` lifetime; this is
/// necessary to make the GAT [`Data::Apply`] usable.
pub trait Data<'bound> {
    /* Nodes ---------------------------------------------------------------- */

    /// An item node in the syntax tree.
    type Item: 'bound;

    /// An expression node in the syntax tree.
    type Expr: 'bound;

    /// A pattern node in the syntax tree.
    type Pattern: 'bound;

    /// A type node in the syntax tree (i.e. a type as it appears in the source,
    /// not an inferred type).
    type Type: 'bound;

    /* Names ---------------------------------------------------------------- */

    /// An unresolved name used in an expression.
    type ExprName: 'bound;

    /// An unresolved name used in a pattern.
    type PatternName: 'bound;

    /// A resolved value-level variable name.
    type Var: 'bound;

    /// A resolved implicitly defined universal type parameter, like `'a`.
    type Universal: 'bound;

    /* Associated data ------------------------------------------------------ */

    /// The representation of an application (such as in an expression or a
    /// pattern).
    type Apply<T: 'bound>: 'bound;

    /// Additional data bound at a generalization scope.
    type GenScope: 'bound;
}

pub enum ItemNode<'bound, D: Data<'bound>> {
    /// Something fishy
    Invalid(ErrorId),

    /// `let a = x`
    Let(D::Pattern, D::Expr, D::GenScope),
}

pub enum ExprNode<'a, 'lit, 'bound, D: Data<'bound>> {
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
    Apply(D::Apply<D::Expr>),

    /// `a => x | b => y`
    Lambda(&'a [(D::Pattern, D::Expr)]),

    /// `let a = x in y`
    Let(D::Pattern, &'a [D::Expr; 2], D::GenScope),
}

pub enum PatternNode<'a, 'bound, D: Data<'bound>> {
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

    /// A constructor name
    Constructor(D::Var),

    /// `a : t`
    Anno(&'a D::Pattern, D::Type),

    /// `(a)`
    Group(&'a D::Pattern),

    /// A pattern application
    Apply(D::Apply<D::Pattern>),
}

pub enum TypeNode<'a, 'lit, 'bound, D: Data<'bound>> {
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

impl<'bound, D: Data<'bound>> Copy for ItemNode<'bound, D>
where
    D::Pattern: Copy,
    D::Expr: Copy,
    D::GenScope: Copy,
{
}

impl<'bound, D: Data<'bound>> Clone for ItemNode<'bound, D>
where
    D::Pattern: Copy,
    D::Expr: Copy,
    D::GenScope: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'bound, D: Data<'bound>> Copy for ExprNode<'_, '_, 'bound, D>
where
    D::Pattern: Copy,
    D::Type: Copy,
    D::ExprName: Copy,
    D::Var: Copy,
    D::Apply<D::Expr>: Copy,
    D::GenScope: Copy,
{
}

impl<'bound, D: Data<'bound>> Clone for ExprNode<'_, '_, 'bound, D>
where
    D::Pattern: Copy,
    D::Type: Copy,
    D::ExprName: Copy,
    D::Var: Copy,
    D::Apply<D::Expr>: Copy,
    D::GenScope: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'bound, D: Data<'bound>> Copy for PatternNode<'_, 'bound, D>
where
    D::Type: Copy,
    D::PatternName: Copy,
    D::Var: Copy,
    D::Apply<D::Pattern>: Copy,
{
}

impl<'bound, D: Data<'bound>> Clone for PatternNode<'_, 'bound, D>
where
    D::Type: Copy,
    D::PatternName: Copy,
    D::Var: Copy,
    D::Apply<D::Pattern>: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'bound, D: Data<'bound>> Copy for TypeNode<'_, '_, 'bound, D> where D::Universal: Copy {}

impl<'bound, D: Data<'bound>> Clone for TypeNode<'_, '_, 'bound, D>
where
    D::Universal: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}
