mod parsing;
mod tokens;

#[cfg(test)]
mod tests;

use self::tokens::Token;
use super::green::Node;
use crate::syntax::green::{Data, Kind};

pub fn parse(source: &str) -> Node {
    parsing::parse(tokenize(source))
}

fn tokenize(source: &str) -> impl Iterator<Item = Node> + '_ {
    use logos::Logos;

    tokens::Token::lexer(source)
        .spanned()
        .map(|(result, range)| {
            let lexeme = &source[range];
            let kind = match result {
                Err(()) => Kind::Invalid,
                Ok(Token::Name) => Kind::Name,
                Ok(Token::PreTick) => Kind::PreTick,
                Ok(Token::PostTick) => Kind::PostTick,
                Ok(Token::Number) => Kind::Number,
                Ok(Token::And) => Kind::And,
                Ok(Token::Case) => Kind::Case,
                Ok(Token::Data) => Kind::Data,
                Ok(Token::End) => Kind::End,
                Ok(Token::In) => Kind::In,
                Ok(Token::Infix) => Kind::Infix,
                Ok(Token::Let) => Kind::Let,
                Ok(Token::Postfix) => Kind::Postfix,
                Ok(Token::Ampersand) => Kind::Ampersand,
                Ok(Token::Comma) => Kind::Comma,
                Ok(Token::Dot) => Kind::Dot,
                Ok(Token::Ellipses) => Kind::Ellipses,
                Ok(Token::Colon) => Kind::Colon,
                Ok(Token::Equal) => Kind::Equal,
                Ok(Token::EqualArrow) => Kind::EqualArrow,
                Ok(Token::Pipe) => Kind::Pipe,
                Ok(Token::Underscore) => Kind::Underscore,
                Ok(Token::LeftParen) => Kind::LeftParen,
                Ok(Token::RightParen) => Kind::RightParen,
                Ok(Token::LeftBrace) => Kind::LeftBrace,
                Ok(Token::RightBrace) => Kind::RightBrace,
                Ok(Token::Comment) => Kind::Comment,
                Ok(Token::Whitespace) => Kind::Whitespace,
            };

            Node {
                width: lexeme.len(),
                kind,
                data: Data::Token(lexeme.into()),
            }
        })
}
