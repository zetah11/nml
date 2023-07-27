pub use tokens::Token;

mod abstractify;
mod cst;
mod parser;
mod tokens;

use bumpalo::Bump;
use logos::Logos;

use self::abstractify::Abstractifier;
use self::parser::Parser;
use crate::errors::Errors;
use crate::names::Names;
use crate::source::{Source, Span};
use crate::trees::parsed;

pub fn tokens(source: &Source) -> impl Iterator<Item = (Result<Token, ()>, Span)> {
    Token::lexer(&source.content)
        .spanned()
        .map(|(result, span)| (result, source.id.span(span.start, span.end)))
}

pub fn parse<'a>(alloc: &'a Bump, names: &'a Names, source: &Source) -> parsed::Program<'a> {
    let tokens = tokens(source);

    let mut errors = Errors::new();
    let concrete_alloc = Bump::new();

    let (concrete, parse_errors) = {
        let parser = Parser::new(&concrete_alloc, &mut errors, tokens, source.id);
        parser.program()
    };

    let (abstracted, unattached) = {
        let abstractifier = Abstractifier::new(alloc, names, &mut errors, parse_errors);
        abstractifier.program(concrete)
    };

    parsed::Program { items: abstracted, errors, unattached, source: source.id }
}
