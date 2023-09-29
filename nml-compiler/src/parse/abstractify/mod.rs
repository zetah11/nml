//! The parser converts a stream of tokens into a bunch of largely unstructured
//! trees. The job of the abstractifier is to then give some more structure to
//! those trees, inserting the difference between items, expressions, patterns,
//! and so on.

mod expr;
mod items;
mod lambda;
mod pattern;
mod types;

use bumpalo::Bump;
use internment::Arena;
use malachite::num::basic::traits::Zero;
use malachite::Integer;

use super::cst;
use crate::errors::{ErrorId, Errors};
use crate::literals::Literal;
use crate::messages::parse::NonSmallName;
use crate::names::{Ident, Names};
use crate::source::Span;
use crate::trees::parsed as ast;

pub struct Abstractifier<'a, 'lit, 'err> {
    alloc: &'a Bump,
    names: &'a Names<'lit>,
    literals: &'lit Arena<Literal>,
    errors: &'err mut Errors,

    parse_errors: Vec<(ErrorId, Span)>,
}

impl<'a, 'lit, 'err> Abstractifier<'a, 'lit, 'err> {
    pub fn new(
        alloc: &'a Bump,
        names: &'a Names<'lit>,
        literals: &'lit Arena<Literal>,
        errors: &'err mut Errors,
        parse_errors: Vec<(ErrorId, Span)>,
    ) -> Self {
        Self {
            alloc,
            names,
            literals,
            errors,
            parse_errors,
        }
    }

    pub fn program(
        mut self,
        items: Vec<&cst::Thing>,
    ) -> (&'a [ast::Item<'a, 'lit>], Vec<(ErrorId, Span)>) {
        let mut into = bumpalo::collections::Vec::with_capacity_in(items.len(), self.alloc);

        for node in items {
            self.item(&mut into, node);
        }

        into.shrink_to_fit();
        (into.into_bump_slice(), self.parse_errors)
    }

    fn normal_name(&mut self, thing: &cst::Thing) -> (Result<Ident<'lit>, ErrorId>, Span) {
        let span = thing.span;
        let ident = match thing.node {
            cst::Node::Invalid(e) => Err(e),
            cst::Node::Name(cst::Name::Normal(name)) => Ok(self.names.intern(name)),

            cst::Node::Name(cst::Name::Universal(name)) => Err(self
                .errors
                .parse_error(span)
                .expected_name_small(NonSmallName::Universal(name))),

            _ => Err(self
                .errors
                .parse_error(span)
                .expected_name_small(NonSmallName::None)),
        };

        (ident, span)
    }

    fn parse_number(&self, lit: &str) -> &'lit Integer {
        let mut res = Integer::ZERO;

        for c in lit.chars() {
            let Some(digit) = c.to_digit(10) else {
                continue;
            };
            res = res * Integer::from(10) + Integer::from(digit);
        }

        Literal::int(self.literals, res)
    }
}
