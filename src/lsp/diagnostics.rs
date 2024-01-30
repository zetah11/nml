use std::collections::{BTreeMap, HashMap};

use lsp_document::{IndexedText, Pos, TextAdapter, TextMap};
use lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString,
    Position, Range, Url,
};

use super::Server;
use crate::frontend::errors::{Error, ErrorType, Errors, Severity};
use crate::frontend::source::{Source, SourceId, Span};
use crate::meta;

impl Server {
    pub fn send_diagnostics(&mut self, errors: &mut Errors) {
        let diagnostics = self.make_diagnostics(errors);

        self.errors = diagnostics
            .iter()
            .filter(|(_, errs)| !errs.is_empty())
            .map(|(url, _)| url.clone())
            .collect();

        for (uri, diagnostics) in diagnostics {
            self.client.publish_diagnostics(uri, diagnostics, None);
        }
    }

    fn make_diagnostics(&self, errors: &mut Errors) -> BTreeMap<Url, Vec<Diagnostic>> {
        DiagnosticBuilder::from_sources(self, errors.sources().collect::<Vec<_>>(), |builder| {
            let mut diagnostics: BTreeMap<Url, Vec<_>> = BTreeMap::new();

            for (_, error) in errors.drain() {
                let (url, diagnostic) = builder.build(error);
                diagnostics.entry(url).or_default().push(diagnostic);
            }

            for url in self.errors.iter() {
                if !diagnostics.contains_key(url) {
                    diagnostics.insert(url.clone(), Vec::new());
                }
            }

            diagnostics
        })
    }
}

struct DiagnosticBuilder<'a> {
    refs: &'a HashMap<SourceId, (&'a Url, &'a Source)>,
    indicies: HashMap<SourceId, IndexedText<&'a str>>,
}

impl<'a> DiagnosticBuilder<'a> {
    pub fn from_sources<F, T>(
        server: &Server,
        sources: impl IntoIterator<Item = SourceId>,
        f: F,
    ) -> T
    where
        F: FnOnce(DiagnosticBuilder) -> T,
    {
        let refs: HashMap<_, _> = sources
            .into_iter()
            .map(|source| {
                let name = server
                    .names
                    .get(&source)
                    .expect("all known source ids correspond to known names");
                let rf = server
                    .tracked
                    .get(name)
                    .expect("all known names correspond to tracked sources");
                (source, (name, rf))
            })
            .collect();

        let indicies = refs
            .iter()
            .map(|(source, rf)| (*source, IndexedText::new(rf.1.content.as_str())))
            .collect();

        f(DiagnosticBuilder {
            refs: &refs,
            indicies,
        })
    }

    pub fn build(&self, error: Error) -> (Url, Diagnostic) {
        let severity = match error.severity {
            Severity::Error => DiagnosticSeverity::ERROR,
            Severity::Warning => DiagnosticSeverity::WARNING,
            Severity::Info => DiagnosticSeverity::INFORMATION,
        };

        let (url, range) = self.span_to_range(error.at);

        let related_information = error
            .labels
            .into_iter()
            .map(|(message, span)| {
                let (url, range) = self.span_to_range(span);
                let location = Location {
                    uri: url.clone(),
                    range,
                };

                DiagnosticRelatedInformation { location, message }
            })
            .collect();

        let code = match error.ty {
            ErrorType::Syntax => "syntax",
            ErrorType::Name => "name",
            ErrorType::Type => "type",
            ErrorType::Evaluation => "eval",
        };

        let diagnostic = Diagnostic {
            range,
            severity: Some(severity),
            code: Some(NumberOrString::String(code.into())),
            code_description: None,
            source: Some(meta::NAME.into()),
            message: error.title,
            related_information: Some(related_information),
            tags: None,
            data: None,
        };

        (url.clone(), diagnostic)
    }

    pub fn span_to_range(&self, span: Span) -> (&'a Url, Range) {
        let text = self
            .indicies
            .get(&span.source)
            .expect("the builder is initialized with all relevant sources");

        let source = self
            .refs
            .get(&span.source)
            .expect("the builder is initialized with all relevant sources");

        let range = text
            .offset_range_to_range(span.start..span.end)
            .unwrap_or(Pos::new(0, 0)..Pos::new(0, 0));
        let range = text.range_to_lsp_range(&range).unwrap_or_default();
        let range = Range {
            start: Position {
                line: range.start.line,
                character: range.start.character,
            },
            end: Position {
                line: range.end.line,
                character: range.end.character,
            },
        };

        (source.0, range)
    }
}
