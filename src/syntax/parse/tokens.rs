use logos::Logos;

#[derive(Logos, Clone, Debug, Eq, PartialEq)]
pub enum Token {
    #[regex(r"[\p{XID_Start}--\p{Other_ID_Start}][\p{XID_Continue}]*")]
    #[regex(r"[\p{Symbol}\p{Punctuation}--[()\[\]{},]]+")]
    Name,

    #[regex(r"'[\p{XID_Start}--\p{Other_ID_Start}][\p{XID_Continue}]*")]
    PreTick,

    #[regex(r"[\p{XID_Start}--\p{Other_ID_Start}][\p{XID_Continue}]*'")]
    PostTick,

    #[regex(r"[0-9][0-9_]*")]
    Number,

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
    #[regex(r"--[^\n]*")]
    Comment,

    #[regex(r"\s+")]
    Whitespace,
}
