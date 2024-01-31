//! Code like
//!
//! ```nml
//! let f = A | B => x | C | D => y
//! ```
//!
//! essentially has two different pipes with different precedences. The pipe
//! between `x` and `C` seperates two case arms, while the pipes between `A` and
//! `B`, and `C` and `D` separates alternatives in a disjunctive pattern. The
//! former binds looser than the arrow while the latter binds tighter.
//!
//! We deal with this by noting that unparenthesized disjunctive patterns can
//! only appear for the first parameter in a chain of arrows. The parser here
//! starts of by creating alternatives when it encounters pipes, transitioning
//! to an "arrow state" when it encounters an arrow. Hitting a pipe in the arrow
//! state creates an arrow and returns to producing disjunctions.
//!
//! We also need to be careful to only introduce an arrow if we actually consume
//! an arrow token. Otherwise, a nested disjunctive pattern or case type would
//! include a non-existent lambda.

use log::trace;

use crate::frontend::parse::cst::{Node, Thing};
use crate::frontend::parse::tokens::Token;
use crate::frontend::source::Span;

use super::Parser;

impl<'a, 'src, 'err, I> Parser<'a, 'src, 'err, I>
where
    I: Iterator<Item = (Result<Token<'src>, ()>, Span)>,
{
    /// ```abnf
    /// lambda = ["|"] simple *(("|" / "=>") simple)
    /// ```
    pub(super) fn lambda(&mut self) -> &'a Thing<'a, 'src> {
        trace!("parse lambda");

        let opener = self.consume(Token::Pipe);
        let expr = self.simple();

        // `true` if this is a single `simple` expression without any pipes or
        // arrows around it. If this is true, the state is `State::Alts` with
        // a single term.
        let mut single_term = opener.is_none();

        let mut state = {
            let span = expr.span;
            let alternatives = vec![expr];
            State::Alts { alternatives, span }
        };

        let mut total_span = opener.unwrap_or(expr.span);
        let mut arrows = Vec::new();

        loop {
            if let Some(pipe) = self.consume(Token::Pipe) {
                total_span += pipe;
                single_term = false;

                state = match state {
                    State::Arrow { patterns, body } => {
                        arrows.push(self.make_arrow(patterns, body));

                        let expr = self.simple();
                        let span = expr.span;
                        let alternatives = vec![expr];
                        State::Alts { alternatives, span }
                    }

                    State::Alts {
                        mut alternatives,
                        mut span,
                    } => {
                        let thing = self.simple();
                        span += thing.span;
                        total_span += thing.span;
                        alternatives.push(thing);
                        State::Alts { alternatives, span }
                    }
                };
            } else if let Some(arrow) = self.consume(Token::EqualArrow) {
                total_span += arrow;
                single_term = false;

                state = match state {
                    State::Arrow {
                        mut patterns,
                        mut body,
                    } => {
                        patterns.push(body);
                        body = self.simple();
                        total_span += body.span;
                        State::Arrow { patterns, body }
                    }

                    State::Alts { alternatives, span } => {
                        let node = Node::Alt(alternatives);
                        let thing = &*self.alloc.alloc(Thing { node, span });
                        let patterns = vec![thing];
                        let body = self.simple();
                        total_span += body.span;
                        State::Arrow { patterns, body }
                    }
                };
            } else {
                break;
            }
        }

        match state {
            State::Arrow { patterns, body } => {
                arrows.push(self.make_arrow(patterns, body));
                let node = Node::Alt(arrows);
                self.alloc.alloc(Thing {
                    node,
                    span: total_span,
                })
            }

            State::Alts { alternatives, span } if arrows.is_empty() => {
                if single_term {
                    debug_assert!(alternatives.len() == 1);
                    alternatives.into_iter().next().unwrap()
                } else {
                    let node = Node::Alt(alternatives);
                    self.alloc.alloc(Thing { node, span })
                }
            }

            State::Alts { alternatives, span } => {
                let pattern = {
                    let node = Node::Alt(alternatives);
                    &*self.alloc.alloc(Thing { node, span })
                };

                let body = {
                    let e = self.errors.parse_error(span).expected_equal_arrow();
                    let node = Node::Invalid(e);
                    &*self.alloc.alloc(Thing { node, span })
                };

                let node = Node::Arrow(pattern, body);
                let span = pattern.span + body.span;
                self.alloc.alloc(Thing { node, span })
            }
        }
    }

    fn make_arrow(
        &self,
        patterns: Vec<&'a Thing<'a, 'src>>,
        mut body: &'a Thing<'a, 'src>,
    ) -> &'a Thing<'a, 'src> {
        for pat in patterns.into_iter().rev() {
            let span = pat.span + body.span;
            let node = Node::Arrow(pat, body);
            body = self.alloc.alloc(Thing { node, span });
        }

        body
    }
}

enum State<'a, 'src> {
    Alts {
        alternatives: Vec<&'a Thing<'a, 'src>>,
        span: Span,
    },

    Arrow {
        patterns: Vec<&'a Thing<'a, 'src>>,
        body: &'a Thing<'a, 'src>,
    },
}
