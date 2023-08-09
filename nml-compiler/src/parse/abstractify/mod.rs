//! The parser converts a stream of tokens into a bunch of largely unstructured
//! trees. The job of the abstractifier is to then give some more structure to
//! those trees, inserting the difference between items, expressions, patterns,
//! and so on.

mod expr;
mod items;
mod lambda;
mod pattern;

use bumpalo::Bump;
use malachite::num::basic::traits::Zero;
use malachite::Integer;

use super::cst;
use crate::errors::{ErrorId, Errors};
use crate::names::{Ident, Names};
use crate::source::Span;
use crate::trees::parsed as ast;

pub struct Abstractifier<'a, 'err> {
    alloc: &'a Bump,
    names: &'a Names<'a>,
    errors: &'err mut Errors,

    parse_errors: Vec<(ErrorId, Span)>,
}

impl<'a, 'err> Abstractifier<'a, 'err> {
    pub fn new(
        alloc: &'a Bump,
        names: &'a Names<'a>,
        errors: &'err mut Errors,
        parse_errors: Vec<(ErrorId, Span)>,
    ) -> Self {
        Self { alloc, names, errors, parse_errors }
    }

    pub fn program(
        mut self,
        items: Vec<&cst::Thing>,
    ) -> (&'a [ast::Item<'a>], Vec<(ErrorId, Span)>) {
        let mut into = bumpalo::collections::Vec::with_capacity_in(items.len(), self.alloc);

        for node in items {
            self.item(&mut into, node);
        }

        into.shrink_to_fit();
        (into.into_bump_slice(), self.parse_errors)
    }

    fn small_name(&mut self, thing: &cst::Thing) -> (Result<Ident, ErrorId>, Span) {
        let span = thing.span;
        let ident = match thing.node {
            cst::Node::Invalid(e) => Err(e),
            cst::Node::Name(cst::Name::Small(name)) => Ok(self.names.intern(name)),
            cst::Node::Name(cst::Name::Operator(name)) => Ok(self.names.intern(name)),

            cst::Node::Name(cst::Name::Big(name)) => {
                Err(self.errors.parse_error(span).expected_name_small(Some(name)))
            }

            _ => Err(self.errors.parse_error(span).expected_name_small(None)),
        };

        (ident, span)
    }

    fn parse_number(lit: &str) -> Integer {
        let mut res = Integer::ZERO;

        for c in lit.chars() {
            let Some(digit) = c.to_digit(10) else { continue; };
            res = res * Integer::from(10) + Integer::from(digit);
        }

        res
    }
}
