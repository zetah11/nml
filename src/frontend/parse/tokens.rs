use logos::Logos;

#[derive(Logos, Clone, Debug, Eq, PartialEq)]
pub enum Token<'src> {
    #[regex(r"[\p{XID_Start}][\p{XID_Continue}_']*", |lexer| lexer.slice(), priority = 2)]
    Name(&'src str),

    #[regex(r"[\p{Symbol}\p{Punctuation}--[()\[\]{},]]+", |lexer| lexer.slice())]
    Symbol(&'src str),

    #[regex(r"'[\p{XID_Start}][\p{XID_Continue}_']*", |lexer| lexer.slice())]
    Universal(&'src str),

    #[regex(r"[0-9][0-9_]*", |lexer| lexer.slice())]
    Number(&'src str),

    #[token("and")]
    And,
    #[token("case")]
    Case,
    #[token("data")]
    Data,
    #[token("end")]
    End,
    #[token("in")]
    In,
    #[token("infix")]
    Infix,
    #[token("let")]
    Let,
    #[token("postfix")]
    Postfix,

    #[token("&")]
    Ampersand,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("...")]
    Ellipses,
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

    // Trivia
    #[regex(r"--[^\n]*", |lexer| lexer.slice())]
    Comment(&'src str),

    #[regex(r"\s+", |lexer| lexer.slice())]
    Whitespace(&'src str),
}
