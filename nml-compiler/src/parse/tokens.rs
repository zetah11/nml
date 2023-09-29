use logos::Logos;

#[derive(Logos, Debug, Eq, PartialEq)]
#[logos(skip r"\s+")]
pub enum Token<'src> {
    #[regex(r"\S+", |lexer| lexer.slice())]
    Name(&'src str),

    #[regex(r"'\S+", |lexer| lexer.slice())]
    Universal(&'src str),

    #[regex(r"[0-9][0-9_']*", |lexer| lexer.slice(), priority = 2)]
    Number(&'src str),

    #[token("and")]
    And,
    #[token("case")]
    Case,
    #[token("do")]
    Do,
    #[token("else")]
    Else,
    #[token("end")]
    End,
    #[token("if")]
    If,
    #[token("in")]
    In,
    #[token("infix")]
    Infix,
    #[token("let")]
    Let,
    #[token("postfix")]
    Postfix,

    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token(":")]
    Colon,
    #[token("=")]
    Equal,
    #[token("=>")]
    EqualArrow,
    #[token("|")]
    Pipe,
    #[token("_")]
    Underscore,

    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,

    #[regex(r"--[^\n]*")]
    Comment,
}
