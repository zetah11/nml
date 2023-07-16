mod cst;
mod parser;
mod tokens;

use bumpalo::Bump;
use logos::Logos;

use self::parser::Parser;
use self::tokens::Token;
use crate::errors::Errors;
use crate::source::SourceId;

pub fn parse(id: SourceId, source: &str) {
    let tokens = Token::lexer(source)
        .spanned()
        .map(|(result, span)| (result, id.span(span.start, span.end)));

    let alloc = Bump::new();
    let mut errors = Errors::new();
    let parser = Parser::new(&alloc, &mut errors, tokens, id);
    let _program = parser.program();

    todo!()
}
