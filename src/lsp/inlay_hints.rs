use lsp_document::{IndexedText, TextMap};
use lsp_types as lsp;

use super::Server;
use crate::frontend::alloc::Bump;
use crate::frontend::names::Names;
use crate::frontend::source::{Source, Span};
use crate::frontend::trees::inferred::{Item, ItemNode, PolyPattern, PolyPatternNode};
use crate::frontend::tyck::{Pretty, Scheme};

impl Server {
    pub fn make_hints(&self, source: &Source) -> Vec<lsp::InlayHint> {
        let alloc = Bump::new();
        let names = Names::new();
        let program = self.check_source(&names, &alloc, source);

        let mut builder = HintsBuilder::new(&names, source.content.as_str());

        for items in program.items {
            builder.items(items);
        }

        builder.hints
    }
}

struct HintsBuilder<'a, 'src> {
    index: IndexedText<&'a str>,
    hints: Vec<lsp::InlayHint>,
    pretty: Pretty<'a, 'src>,
}

impl<'a, 'src> HintsBuilder<'a, 'src> {
    pub fn new(names: &'a Names<'src>, text: &'a str) -> Self {
        Self {
            index: IndexedText::new(text),
            hints: Vec::new(),
            pretty: Pretty::new(names),
        }
    }

    pub fn items(&mut self, items: &[Item]) {
        for item in items {
            match &item.node {
                ItemNode::Invalid(_) => {}
                ItemNode::Let(pattern, _, _) => {
                    self.pattern(pattern);
                }
                ItemNode::Data(_, _) => {}
            }
        }
    }

    fn pattern(&mut self, pattern: &PolyPattern) {
        match &pattern.node {
            PolyPatternNode::Invalid(_)
            | PolyPatternNode::Wildcard
            | PolyPatternNode::Unit
            | PolyPatternNode::Constructor(_) => {}

            PolyPatternNode::Bind(_) => {
                self.hint_scheme(pattern.span, &pattern.scheme);
            }

            PolyPatternNode::Group(pattern) => self.pattern(pattern),

            PolyPatternNode::Apply([a, b])
            | PolyPatternNode::Or([a, b])
            | PolyPatternNode::And([a, b]) => {
                self.pattern(a);
                self.pattern(b);
            }

            PolyPatternNode::Anno(_, v) => match *v {},
        }
    }

    fn hint_scheme(&mut self, at: Span, scheme: &Scheme) {
        let Some(position) = self.span_to_end_position(at) else {
            return;
        };

        let ty = {
            let mut pretty = self.pretty.build();
            format!(": {}", pretty.scheme(scheme))
        };

        let hint = lsp::InlayHint {
            position,
            label: lsp::InlayHintLabel::String(ty),
            kind: Some(lsp::InlayHintKind::TYPE),
            text_edits: None,
            tooltip: None,
            padding_left: Some(true),
            padding_right: Some(false),
            data: None,
        };

        self.hints.push(hint);
    }

    fn span_to_end_position(&self, span: Span) -> Option<lsp::Position> {
        self.index.offset_to_pos(span.end).map(|pos| lsp::Position {
            line: pos.line,
            character: pos.col,
        })
    }
}
