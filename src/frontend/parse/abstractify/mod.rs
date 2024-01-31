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

use super::cst;
use crate::frontend::errors::{ErrorId, Errors};
use crate::frontend::names::{Ident, Names};
use crate::frontend::source::Span;
use crate::frontend::trees::parsed as ast;

pub struct Abstractifier<'a, 'src, 'err> {
    alloc: &'a Bump,
    names: &'a Names<'src>,
    errors: &'err mut Errors,

    parse_errors: Vec<(ErrorId, Span)>,
}

impl<'a, 'src, 'err> Abstractifier<'a, 'src, 'err> {
    pub fn new(
        alloc: &'a Bump,
        names: &'a Names<'src>,
        errors: &'err mut Errors,
        parse_errors: Vec<(ErrorId, Span)>,
    ) -> Self {
        Self {
            alloc,
            names,
            errors,
            parse_errors,
        }
    }

    pub fn program(
        mut self,
        items: Vec<&cst::Thing<'_, 'src>>,
    ) -> (&'a [ast::Item<'a, 'src>], Vec<(ErrorId, Span)>) {
        let mut into = bumpalo::collections::Vec::with_capacity_in(items.len(), self.alloc);

        for node in items {
            self.item(&mut into, node);
        }

        into.shrink_to_fit();
        (into.into_bump_slice(), self.parse_errors)
    }

    fn normal_name(
        &mut self,
        thing: &cst::Thing<'_, 'src>,
    ) -> (Result<Ident<'src>, ErrorId>, Span) {
        let span = thing.span;
        let ident = match thing.node {
            cst::Node::Invalid(e) => Err(e),
            cst::Node::Name(cst::Name::Normal(name)) => Ok(self.names.intern(name)),

            cst::Node::Name(cst::Name::Universal(name)) => Err(self
                .errors
                .parse_error(span)
                .expected_non_universal_name(name)),

            _ => Err(self.errors.parse_error(span).expected_name()),
        };

        (ident, span)
    }
}
