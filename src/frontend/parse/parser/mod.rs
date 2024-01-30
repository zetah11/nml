mod lambda;
mod things;

use bumpalo::Bump;

use super::cst::Thing;
use super::tokens::Token;
use crate::frontend::errors::{ErrorId, Errors};
use crate::frontend::source::{SourceId, Span};

pub struct Parser<'a, 'err, I: Iterator<Item = (Result<Token<'a>, ()>, Span)>> {
    tokens: I,
    alloc: &'a Bump,

    current_span: Span,
    next: Option<(Token<'a>, Span)>,
    prev: Option<(Token<'a>, Span)>,

    errors: &'err mut Errors,
    parse_errors: Vec<(ErrorId, Span)>,
}

impl<'a, 'err, I: Iterator<Item = (Result<Token<'a>, ()>, Span)>> Parser<'a, 'err, I> {
    pub fn new(alloc: &'a Bump, errors: &'err mut Errors, tokens: I, source: SourceId) -> Self {
        let current_span = source.span(0, 0);
        Self {
            tokens,
            alloc,
            current_span,
            next: None,
            prev: None,
            errors,
            parse_errors: Vec::new(),
        }
    }

    /// Parse an entire source.
    pub fn program(mut self) -> (Vec<&'a Thing<'a>>, Vec<(ErrorId, Span)>) {
        self.advance();
        let result = self.top_level();
        (result, self.parse_errors)
    }

    /// Get the span closest to the next token.
    fn closest_span(&mut self) -> Span {
        self.next
            .as_ref()
            .or(self.prev.as_ref())
            .map(|(_, span)| *span)
            .unwrap_or(self.current_span)
    }

    /// Move to the next token.
    fn advance(&mut self) {
        self.prev = self.next.take();
        let mut erred = false;
        for (token, span) in self.tokens.by_ref() {
            if let Ok(token) = token {
                if let Token::Comment = token {
                    continue;
                }

                self.next = Some((token, span));
                self.current_span = span;
                break;
            } else if !erred {
                let e = self.errors.parse_error(span).unexpected_token();
                self.parse_errors.push((e, span));
                erred = true;
            }
        }
    }

    /// Returns `Some(next_span)` if the next token matches.
    fn peek(&self, m: impl Matcher) -> Option<Span> {
        self.next
            .as_ref()
            .and_then(|(token, span)| m.matches(token).then_some(*span))
    }

    /// Advances the parser and returns `Some(span)` if the token which was next
    /// matched. If it did not, then the parser is unaffected. The returned span
    /// is from the token which was matched.
    fn consume(&mut self, m: impl Matcher) -> Option<Span> {
        self.peek(m).map(|span| {
            self.advance();
            span
        })
    }
}

trait Matcher {
    fn matches(&self, token: &Token) -> bool;
}

impl Matcher for Token<'_> {
    /// Returns `true` if `self` and `token` are the same token kind, even if
    /// they disagree on content.
    fn matches(&self, token: &Token) -> bool {
        match (self, token) {
            (Token::Name(_), Token::Name(_)) => true,
            (Token::Symbol(_), Token::Symbol(_)) => true,
            (Token::Universal(_), Token::Universal(_)) => true,
            (Token::Number(_), Token::Number(_)) => true,

            _ => self == token,
        }
    }
}

impl<M: Matcher> Matcher for &'_ [M] {
    /// Returns `true` if any of the contained matchers mathces the given token.
    fn matches(&self, token: &Token) -> bool {
        self.iter().any(|matcher| matcher.matches(token))
    }
}

impl<F: Fn(&Token) -> bool> Matcher for F {
    /// Returns `true` if the given function returns `true`.
    fn matches(&self, token: &Token) -> bool {
        self(token)
    }
}
