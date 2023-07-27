use nml_compiler::parse::Token;
use tower_lsp::lsp_types::{SemanticTokenModifier, SemanticTokenType, SemanticTokensLegend};

pub fn get() -> SemanticTokensLegend {
    SemanticTokensLegend { token_types: TYPES.into(), token_modifiers: MODIFIERS.into() }
}

/// Get the semantic token type and the modifiers for the given token type,
/// if it is to be highlighted.
pub fn for_token(token: Result<Token, ()>) -> Option<(u32, u32)> {
    let Ok(token) = token else {
        return None;
    };

    match token {
        Token::BigName(_) | Token::SmallName(_) => None,

        Token::Number(_) => Some((types::NUMBER, mods::NONE)),

        Token::And
        | Token::Case
        | Token::Do
        | Token::Else
        | Token::End
        | Token::Fun
        | Token::If
        | Token::In
        | Token::Let => Some((types::KEYWORD, mods::NONE)),

        Token::Comma
        | Token::Dot
        | Token::Equal
        | Token::EqualArrow
        | Token::Pipe
        | Token::Underscore => None,

        Token::LeftParen | Token::RightParen | Token::LeftBrace | Token::RightBrace => None,
    }
}

const TYPES: [SemanticTokenType; 4] = [
    SemanticTokenType::COMMENT,
    SemanticTokenType::NUMBER,
    SemanticTokenType::STRING,
    SemanticTokenType::KEYWORD,
];

const MODIFIERS: [SemanticTokenModifier; 1] = [SemanticTokenModifier::DOCUMENTATION];

mod types {
    //pub const COMMENT: u32 = 0;
    pub const NUMBER: u32 = 1;
    //pub const STRING: u32 = 2;
    pub const KEYWORD: u32 = 3;
}

mod mods {
    pub const NONE: u32 = 0;
    //pub const DOCUMENTATION: u32 = 1;
}
