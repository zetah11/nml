use log::debug;
pub use tokens::Token;

mod abstractify;
mod cst;
mod parser;
mod tokens;

use bumpalo::Bump;
use logos::Logos;

use self::abstractify::Abstractifier;
use self::parser::Parser;
use crate::frontend::errors::Errors;
use crate::frontend::names::Names;
use crate::frontend::source::{Source, Span};
use crate::frontend::trees::parsed;

pub fn tokens(source: &Source) -> impl Iterator<Item = (Result<Token, ()>, Span)> {
    Token::lexer(&source.content)
        .spanned()
        .map(|(result, span)| (result, source.id.span(span.start, span.end)))
}

pub fn parse<'a, 'src>(
    alloc: &'a Bump,
    names: &'a Names<'src>,
    source: &'src Source,
) -> parsed::Source<'a, 'src> {
    debug!("lexing");
    let tokens = tokens(source);

    debug!("parsing");
    let mut errors = Errors::new();
    let concrete_alloc = Bump::new();

    let (concrete, parse_errors) = {
        let parser = Parser::new(&concrete_alloc, &mut errors, tokens, source.id);
        parser.program()
    };

    debug!("abstracting");
    let (abstracted, unattached) = {
        let abstractifier = Abstractifier::new(alloc, names, &mut errors, parse_errors);
        abstractifier.program(concrete)
    };

    parsed::Source {
        items: abstracted,
        errors,
        unattached,
        source: source.id,
    }
}
