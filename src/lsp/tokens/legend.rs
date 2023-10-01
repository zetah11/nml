use lsp_types::{SemanticTokenModifier, SemanticTokenType, SemanticTokensLegend};
use nml_compiler::parse::Token;

pub fn get() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TYPES.into(),
        token_modifiers: MODIFIERS.into(),
    }
}

/// Get the semantic token type and the modifiers for the given token type,
/// if it is to be highlighted.
pub fn for_token(token: Result<Token, ()>) -> Option<(u32, u32)> {
    let Ok(token) = token else {
        return None;
    };

    match token {
        Token::Comment => Some((types::COMMENT, mods::NONE)),
        Token::Name(_) | Token::Symbol(_) => None,
        Token::Universal(_) => Some((types::TYPE_PARAMETER, mods::NONE)),

        Token::Number(_) => Some((types::NUMBER, mods::NONE)),

        Token::And
        | Token::Case
        | Token::Do
        | Token::Else
        | Token::End
        | Token::If
        | Token::In
        | Token::Infix
        | Token::Let
        | Token::Postfix => Some((types::KEYWORD, mods::NONE)),

        Token::Comma
        | Token::Dot
        | Token::Colon
        | Token::Equal
        | Token::EqualArrow
        | Token::Pipe
        | Token::Underscore => Some((types::OPERATOR, mods::NONE)),

        Token::LeftParen | Token::RightParen | Token::LeftBrace | Token::RightBrace => None,
    }
}

const TYPES: [SemanticTokenType; 6] = [
    SemanticTokenType::COMMENT,
    SemanticTokenType::NUMBER,
    SemanticTokenType::STRING,
    SemanticTokenType::KEYWORD,
    SemanticTokenType::OPERATOR,
    SemanticTokenType::TYPE_PARAMETER,
];

const MODIFIERS: [SemanticTokenModifier; 1] = [SemanticTokenModifier::DOCUMENTATION];

mod types {
    pub const COMMENT: u32 = 0;
    pub const NUMBER: u32 = 1;
    //pub const STRING: u32 = 2;
    pub const KEYWORD: u32 = 3;
    pub const OPERATOR: u32 = 4;
    pub const TYPE_PARAMETER: u32 = 5;
}

mod mods {
    pub const NONE: u32 = 0;
    //pub const DOCUMENTATION: u32 = 1;
}
