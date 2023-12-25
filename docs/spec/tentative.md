# Tentative language reference

This guide is intended as a "sketched out" overview of the language.

nml is a pure and eager general-purpose functional language. It consists of
five distinct lexical entities:

- Items
- Datatype definitions
- Types
- Patterns
- Expressions

## Items

Items are pure, unordered, "top-level" entities.

```reference
Items = Item*

Item = "let" PatternOrSpine "=" Expression
     | "fun" PatternOrSpine "=" Expression
     | "data" TypeSpine "=" DatatypeDefinition
     | "type" TypeSpine "=" Type
```

`let` and `fun` items bind the result of evaluating an expression to a pattern.
The left-hand pattern must be exhaustive. As a shorthand, a spine can be used
instead of a pattern. In this case, the spine head is bound to a lambda.

`let`s are _shadowing_ while `fun`s are _recursive_. The right-hand expression
of a `let` is not able to refer to the names defined by that item. `fun`s are
recursive, and the bound names can be referred to in the expression.

`type`s are shadowing and `data`s are recursive.

All items can be mutually recursive, in any order. This makes it possible to
have recursive `let` and `type` definitions even if they cannot directly refer
to themselves:

```nml
let inf1 x = inf2 x
let inf2 x = inf1 x

type less1 = less2
data less2 = Recurse less1
```

`type` introduces a type alias: at use sites, type aliases are fully expanded.
`data` introduces a new datatype. Type equality between datatypes is nominal:
two types `d1 t1 t2 ... tN` and `d2 u1 u2 ... uN` are equal if `d1` is nominally
equal to `d2` and `t1` and `u1` through `tN` and `uN` are equal.

It is an error for a set of type alises to form a cycle. Otherwise, expanding
aliases might never terminate.

```nml
type ok1 = ok2
data ok2 = Recurse (list ok1)

type bad1 = bad2
type bad2 = list bad1
```

## Data type definitions

A datatype definition is either a sum type or an effect type definition.

```reference
DatatypeDefinition = "case" ("|" ConstructorDefinition)* "end"
                   | "|"? ConstructorDefinition ("|" ConstructorDefinition)*
                   | "effect" ("|" OperationDefinition)* "end"

ConstructorDefinition = Affix? Name SimpleType*
OperationDefinition = Spine ":" Type
```

## Types

## Patterns

### Spines

A spine is a function name, called the spine head, followed by a set of
parameter patterns. These are used to declare function-like things.

```grammar
Spine = Affix? Name SimplePattern*
```

## Expressions
