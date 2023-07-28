use logos::Logos;

#[derive(Logos, Debug, Eq, PartialEq)]
#[logos(skip r"\s+")]
pub enum Token<'src> {
    #[regex(r"\p{Lowercase_Letter}[\p{XID_Continue}_']*", |lexer| lexer.slice())]
    SmallName(&'src str),

    #[regex(r"\p{Uppercase_Letter}[\p{XID_Continue}_']*", |lexer| lexer.slice())]
    BigName(&'src str),

    #[regex(r"[0-9][0-9_']*", |lexer| lexer.slice())]
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
    #[token("fun")]
    Fun,
    #[token("if")]
    If,
    #[token("in")]
    In,
    #[token("let")]
    Let,

    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
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