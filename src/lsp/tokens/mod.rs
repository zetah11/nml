pub mod legend;

use lsp_types::{SemanticToken, SemanticTokens};
use nml_compiler::parse::{tokens, Token};
use nml_compiler::source::{Source, Span};

use super::Server;

impl Server {
    pub fn compute_tokens(&self, source: &Source) -> SemanticTokens {
        let mut builder = SemanticTokensBuilder::new(&source.content);

        for (token, span) in tokens(source) {
            builder.add_token(token, span);
        }

        builder.build()
    }
}

/// Algorithm: in order to avoid having to traverse the entire source for every
/// token, we store the result of the previous `start_byte -> line, column`
/// mapping, and only retraverse if the given span is less than the start byte.
/// Otherwise, we just traverse the string from the start_byte and compute a
/// delta.
struct SemanticTokensBuilder<'s> {
    source: &'s str,
    tokens: Vec<SemanticToken>,

    /// The start index of the previous span we traversed.
    previous: Option<usize>,
}

struct RelativeSpan {
    delta_line: usize,
    delta_column: usize,
    length: usize,
}

impl<'s> SemanticTokensBuilder<'s> {
    pub fn new(source: &'s str) -> Self {
        Self {
            source,
            tokens: Vec::new(),
            previous: None,
        }
    }

    pub fn build(self) -> SemanticTokens {
        SemanticTokens {
            result_id: None,
            data: self.tokens,
        }
    }

    /// Add a token to this list of semantic tokens.
    pub fn add_token(&mut self, token: Result<Token, ()>, span: Span) {
        if let Some((ty, modifiers)) = legend::for_token(token) {
            let relative = self.translate_span(span);
            self.previous = Some(span.start);
            self.push_token(relative, ty, modifiers);
        }
    }

    /// Push the given type and modifiers at the given span to the token list.
    fn push_token(&mut self, relative: RelativeSpan, ty: u32, modifiers: u32) {
        // TODO: split into multiple if length is too big
        let Ok(delta_line) = u32::try_from(relative.delta_line) else {
            return;
        };
        let Ok(delta_start) = u32::try_from(relative.delta_column) else {
            return;
        };
        let Ok(length) = u32::try_from(relative.length) else {
            return;
        };

        self.tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: ty,
            token_modifiers_bitset: modifiers,
        });
    }

    /// Translate the given `Span` into a span relative to `self.previous`.
    /// This assumes that the given span belongs to the source in `self.source`.
    /// This does *not* replace `self.previous`.
    fn translate_span(&self, span: Span) -> RelativeSpan {
        let (source, mut index) = match self.previous {
            Some(previous) if previous <= span.start => (&self.source[previous..], previous),
            _ => (self.source, 0),
        };

        let mut delta_line = 0;
        let mut delta_column = 0;

        for c in source.chars() {
            if index >= span.start {
                break;
            }

            index += c.len_utf8();

            if c == '\n' {
                delta_line += 1;
                delta_column = 0;
            } else {
                delta_column += 1;
            }
        }

        RelativeSpan {
            delta_line,
            delta_column,
            length: span.length(),
        }
    }
}
