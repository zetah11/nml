use lsp_document::{IndexedText, TextMap};
use lsp_types as lsp;
use nml_compiler::alloc::Bump;
use nml_compiler::names::Names;
use nml_compiler::source::{Source, Span};
use nml_compiler::trees::inferred::{Item, ItemNode, PolyPattern, PolyPatternNode};
use nml_compiler::tyck::{Pretty, Scheme};

use super::Server;

impl Server {
    pub fn make_hints(&self, source: &Source) -> Vec<lsp::InlayHint> {
        let alloc = Bump::new();
        let names = Names::new(&self.idents);
        let program = self.check_source(&names, &alloc, source);

        let mut builder = HintsBuilder::new(&names, source.content.as_str());

        for items in program.items {
            builder.items(items);
        }

        builder.hints
    }
}

struct HintsBuilder<'a, 'lit> {
    index: IndexedText<&'a str>,
    hints: Vec<lsp::InlayHint>,
    pretty: Pretty<'a, 'lit>,
}

impl<'a, 'lit> HintsBuilder<'a, 'lit> {
    pub fn new(names: &'a Names<'lit>, text: &'a str) -> Self {
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
            }
        }
    }

    fn pattern(&mut self, pattern: &PolyPattern) {
        match &pattern.node {
            PolyPatternNode::Invalid(_)
            | PolyPatternNode::Wildcard
            | PolyPatternNode::Unit
            | PolyPatternNode::Named(_) => {}

            PolyPatternNode::Bind(_) => {
                self.hint_scheme(pattern.span, &pattern.scheme);
            }

            PolyPatternNode::Anno(..) => {}

            PolyPatternNode::Deconstruct(_, pattern) => {
                self.pattern(pattern);
            }

            PolyPatternNode::Apply([fun, arg]) => {
                self.pattern(fun);
                self.pattern(arg);
            }

            PolyPatternNode::Small(v) | PolyPatternNode::Big(v) => match *v {},
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
