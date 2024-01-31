//! Defines the various syntax tree variants or "nodes".
//!
//! The types are parameterized by:
//!
//! - `Item` - item trees
//! - `Expr` - expression trees
//! - `Pattern` - pattern trees
//! - `Type` - type trees (but not the _semantic objects_ of types, themselves)
//! - `Var` - a value name
//! - `Constructor` - a resolved constrcutor name
//! - `Universal` - a resolved, implicitly defined universal type parameter,
//!   such as `'a`
//! - `ApplyExpr` - an expression application tree
//! - `ApplyPattern` - a pattern application tree
//! - `GenScope` - data bound at generalizing nodes, like `let` items and
//!   expressions.

use crate::frontend::errors::ErrorId;
use crate::frontend::names::Label;
use crate::frontend::source::Span;

pub enum ItemNode<Expr, Pattern, DataPattern, DataBody, GenScope> {
    /// Something fishy
    Invalid(ErrorId),

    /// `let a = x`
    Let(Pattern, Expr, GenScope),

    /// `data a = t`
    Data(DataPattern, DataBody),
}

pub enum ExprNode<'a, 'src, Expr, Pattern, Type, Name, ApplyExpr, GenScope> {
    /// Something fishy
    Invalid(ErrorId),

    /// Variable name
    Var(Name),

    /// `_`
    Hole,

    /// `()`
    Unit,

    /// Some integer
    Number(&'src str),

    /// `x : t`
    Anno(&'a Expr, Type),

    /// `(x)`
    Group(&'a Expr),

    /* Records -------------------------------------------------------------- */
    /// `x.a`
    Field(&'a Expr, Result<Label<'src>, ErrorId>, Span),

    /// `{ a = x, b = y, ...r }`
    Record(
        &'a [(Result<Label<'src>, ErrorId>, Span, Expr)],
        Option<&'a Expr>,
    ),

    /// `x \ a`
    Restrict(&'a Expr, Label<'src>),

    /* Functions ------------------------------------------------------------ */
    /// `x y`
    Apply(ApplyExpr),

    /// `a => x | b => y`
    Lambda(&'a [(Pattern, Expr)]),

    /// `let a = x in y`
    Let(Pattern, &'a [Expr; 2], GenScope),
}

pub enum PatternNode<'a, Pattern, Type, Name, ConstructorName, ApplyPattern> {
    /// Something fishy.
    Invalid(ErrorId),

    /// `_`
    Wildcard,

    /// `()`
    Unit,

    /// A name binding
    Bind(Name),

    /// A constructor name
    Constructor(ConstructorName),

    /// `a : t`
    Anno(&'a Pattern, Type),

    /// `(a)`
    Group(&'a Pattern),

    /// A pattern application
    Apply(ApplyPattern),

    /// Either the first or second pattern.
    Or(&'a [Pattern; 2]),

    /// The first and then the second pattern.
    And(&'a [Pattern; 2]),
}

pub enum TypeNode<'a, 'src, Type, Name, Universal, ApplyType> {
    /// Bad stuff.
    Invalid(ErrorId),

    /// `_`
    Wildcard,

    /// `abc`
    Named(Name),

    /// `'a`
    Universal(Universal),

    /// `t -> u`
    Function(&'a [Type; 2]),

    /// `{ a : t, b : u }`
    Record(&'a [(Result<Label<'src>, ErrorId>, Span, Type)]),

    /// `(t)`
    Group(&'a Type),

    /// `t u`
    Apply(ApplyType),
}

pub enum DataNode<'a, Constructor> {
    /// Some erroneous data body.
    Invalid(ErrorId),

    /// `C t | D u`
    Sum(&'a [Constructor]),
}

pub enum ConstructorNode<'a, Name, Type> {
    /// Oopsies
    Invalid(ErrorId),

    /// `Some t`
    Constructor(Name, &'a [Type]),
}

/* Copy and Clone impls ----------------------------------------------------- */

impl<Pattern, Expr, TypePattern, DataBody, GenScope> Copy
    for ItemNode<Pattern, Expr, TypePattern, DataBody, GenScope>
where
    Pattern: Copy,
    Expr: Copy,
    TypePattern: Copy,
    DataBody: Copy,
    GenScope: Copy,
{
}

impl<Pattern, Expr, TypePattern, DataBody, GenScope> Clone
    for ItemNode<Pattern, Expr, TypePattern, DataBody, GenScope>
where
    Pattern: Copy,
    Expr: Copy,
    TypePattern: Copy,
    DataBody: Copy,
    GenScope: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Expr, Pattern, Type, Name, ApplyExpr, GenScope> Copy
    for ExprNode<'_, '_, Expr, Pattern, Type, Name, ApplyExpr, GenScope>
where
    Pattern: Copy,
    Type: Copy,
    Name: Copy,
    ApplyExpr: Copy,
    GenScope: Copy,
{
}

impl<Expr, Pattern, Type, Name, ApplyExpr, GenScope> Clone
    for ExprNode<'_, '_, Expr, Pattern, Type, Name, ApplyExpr, GenScope>
where
    Pattern: Copy,
    Type: Copy,
    Name: Copy,
    ApplyExpr: Copy,
    GenScope: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Pattern, Type, PatternName, Name, ApplyPattern> Copy
    for PatternNode<'_, Pattern, Type, PatternName, Name, ApplyPattern>
where
    Type: Copy,
    PatternName: Copy,
    Name: Copy,
    ApplyPattern: Copy,
{
}

impl<Pattern, Type, PatternName, Name, ApplyPattern> Clone
    for PatternNode<'_, Pattern, Type, PatternName, Name, ApplyPattern>
where
    Type: Copy,
    PatternName: Copy,
    Name: Copy,
    ApplyPattern: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Type, Name, Universal, ApplyType> Copy for TypeNode<'_, '_, Type, Name, Universal, ApplyType>
where
    Name: Copy,
    Universal: Copy,
    ApplyType: Copy,
{
}

impl<Type, Name, Universal, ApplyType> Clone for TypeNode<'_, '_, Type, Name, Universal, ApplyType>
where
    Name: Copy,
    Universal: Copy,
    ApplyType: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Constructor> Copy for DataNode<'_, Constructor> where Constructor: Copy {}

impl<Constructor> Clone for DataNode<'_, Constructor>
where
    Constructor: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<Name, Type> Copy for ConstructorNode<'_, Name, Type>
where
    Name: Copy,
    Type: Copy,
{
}

impl<Name, Type> Clone for ConstructorNode<'_, Name, Type>
where
    Name: Copy,
    Type: Copy,
{
    fn clone(&self) -> Self {
        *self
    }
}

/// The point of this module is just to quickly catch any accidental invariances
/// (or contravariances, though that's unlikely) introduced when modifying the
/// types in [`crate::trees::nodes`].
///
/// Covariance is useful when dealing with lifetimes, but things being invariant
/// for no clear reason keeps troubling me. This module exists as a canary to
/// alert whenever things are no longer the variance they should be.
#[allow(unused)]
mod variance_checking {
    use std::convert::Infallible;
    use std::marker::PhantomData;

    use super::{ExprNode, ItemNode, PatternNode, TypeNode};

    struct Item<'a, 'src>(
        ItemNode<Expr<'a, 'src>, Pattern<'a, 'src>, Infallible, Infallible, Infallible>,
    );

    struct Expr<'a, 'src>(
        ExprNode<
            'a,
            'src,
            Self,
            Pattern<'a, 'src>,
            Type<'a, 'src>,
            Infallible,
            Infallible,
            Infallible,
        >,
    );

    struct Pattern<'a, 'src>(
        PatternNode<'a, Self, Type<'a, 'src>, Infallible, Infallible, Infallible>,
    );

    struct Type<'a, 'src>(TypeNode<'a, 'src, Self, Infallible, Infallible, Infallible>);

    fn assert_item_covariance<'a: 'b, 'b, 'src>(v: Item<'a, 'src>) -> Item<'b, 'src> {
        v
    }

    fn assert_expr_covariance<'a: 'b, 'b, 'src>(v: Expr<'a, 'src>) -> Expr<'b, 'src> {
        v
    }

    fn assert_pattern_covariance<'a: 'b, 'b, 'src>(v: Pattern<'a, 'src>) -> Pattern<'b, 'src> {
        v
    }

    fn assert_type_covariance<'a: 'b, 'b, 'src>(v: Type<'a, 'src>) -> Type<'b, 'src> {
        v
    }
}
