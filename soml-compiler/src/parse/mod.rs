mod abstractify;
mod cst;
mod parser;
mod tokens;

use bumpalo::Bump;
use logos::Logos;

use self::abstractify::Abstractifier;
use self::parser::Parser;
use self::tokens::Token;
use crate::errors::Errors;
use crate::names::Names;
use crate::source::SourceId;
use crate::trees::parsed;

pub fn parse<'a>(
    alloc: &'a Bump,
    names: &'a Names,
    id: SourceId,
    source: &str,
) -> parsed::Program<'a> {
    let tokens = Token::lexer(source)
        .spanned()
        .map(|(result, span)| (result, id.span(span.start, span.end)));

    let mut errors = Errors::new();
    let concrete_alloc = Bump::new();

    let (concrete, parse_errors) = {
        let parser = Parser::new(&concrete_alloc, &mut errors, tokens, id);
        parser.program()
    };

    let (abstracted, unattached) = {
        let abstractifier = Abstractifier::new(alloc, names, &mut errors, parse_errors);
        abstractifier.program(concrete)
    };

    parsed::Program { items: abstracted, errors, unattached }
}
