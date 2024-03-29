pub use self::lsp_error::LspError;

mod check;
mod diagnostics;
mod framework;
mod inlay_hints;
mod log;
mod lsp_error;
mod sync;
mod tokens;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bumpalo::Bump;
use lsp::TraceValue;
use lsp_types::{self as lsp, Url};

use self::framework::{Client, Error};
use self::log::{AtomicTraceValue, Logger};
use crate::frontend::names::Names;
use crate::frontend::source::{Source, SourceId, Sources};
use crate::meta;

pub fn run() -> Result<(), LspError> {
    framework::stdio(Builder::new())
}

struct Builder {
    trace: Option<TraceValue>,
}

impl Builder {
    fn new() -> Self {
        Self { trace: None }
    }
}

impl framework::Builder for Builder {
    fn build(self, trace: Arc<AtomicTraceValue>, client: Client) -> Server {
        Logger::init(trace.clone(), client.clone());
        Server::new(client)
    }

    fn initialize(&mut self, params: lsp::InitializeParams) -> lsp::InitializeResult {
        self.trace = params.trace;

        let server_info = Some(lsp::ServerInfo {
            name: meta::NAME.into(),
            version: Some(meta::VERSION.into()),
        });

        let capabilities = lsp::ServerCapabilities {
            inlay_hint_provider: Some(lsp::OneOf::Right(
                lsp::InlayHintServerCapabilities::RegistrationOptions(
                    lsp::InlayHintRegistrationOptions {
                        ..Default::default()
                    },
                ),
            )),

            semantic_tokens_provider: Some(
                lsp::SemanticTokensServerCapabilities::SemanticTokensOptions(
                    lsp::SemanticTokensOptions {
                        legend: tokens::legend::get(),
                        full: Some(lsp::SemanticTokensFullOptions::Bool(true)),
                        ..Default::default()
                    },
                ),
            ),
            text_document_sync: Some(lsp::TextDocumentSyncCapability::Kind(
                lsp::TextDocumentSyncKind::FULL,
            )),
            ..Default::default()
        };

        lsp::InitializeResult {
            server_info,
            capabilities,
        }
    }
}

struct Server {
    client: Client,

    tracked: HashMap<Url, Source>,
    names: HashMap<SourceId, Url>,
    sources: Sources,

    errors: HashSet<Url>,
}

impl Server {
    fn new(client: Client) -> Self {
        Self {
            client,
            tracked: HashMap::new(),
            names: HashMap::new(),
            sources: Sources::new(),

            errors: HashSet::new(),
        }
    }
}

/// Protocol impl
impl Server {
    /// `textDocument/didChange`
    fn did_change_text_document(&mut self, mut params: lsp::DidChangeTextDocumentParams) {
        let name = params.text_document.uri;
        assert_eq!(1, params.content_changes.len(), "full synchronization");

        let text = params.content_changes.remove(0).text;
        self.insert_document(name.clone(), text);

        let mut errors = {
            let alloc = Bump::new();
            let names = Names::new();
            let source = self.tracked.get(&name).expect("just inserted");
            self.check_source(&names, &alloc, source).errors
        };

        self.send_diagnostics(&mut errors);
    }

    /// `textDocument/didOpen`
    fn did_open_text_document(&mut self, params: lsp::DidOpenTextDocumentParams) {
        let name = params.text_document.uri;
        let text = params.text_document.text;
        self.insert_document(name.clone(), text);

        let mut errors = {
            let alloc = Bump::new();
            let names = Names::new();
            let source = self.tracked.get(&name).expect("just inserted");
            self.check_source(&names, &alloc, source).errors
        };

        self.send_diagnostics(&mut errors);
    }

    /// `textDocument/didSave`
    fn did_save_text_document(&mut self, params: lsp::DidSaveTextDocumentParams) {
        if let Some(text) = params.text {
            let name = params.text_document.uri;
            self.insert_document(name.clone(), text);

            let mut errors = {
                let alloc = Bump::new();
                let names = Names::new();
                let source = self.tracked.get(&name).expect("just inserted");
                self.check_source(&names, &alloc, source).errors
            };

            self.send_diagnostics(&mut errors);
        }
    }

    /// `textDocument/semanticTokens/full`
    fn semantic_tokens_full(
        &mut self,
        params: lsp::SemanticTokensParams,
    ) -> Result<Option<lsp::SemanticTokensResult>, Error> {
        let document = {
            let name = params.text_document.uri;
            self.tracked
                .get(&name)
                .ok_or_else(|| Error::InvalidRequest(format!("unknown document `{name}`")))?
        };

        Ok(Some(lsp::SemanticTokensResult::Tokens(
            self.compute_tokens(document),
        )))
    }

    /// `textDocument/inlayHints`
    fn inlay_hints(
        &mut self,
        params: lsp::InlayHintParams,
    ) -> Result<Option<Vec<lsp::InlayHint>>, Error> {
        let name = params.text_document.uri;

        let source = self
            .tracked
            .get(&name)
            .ok_or_else(|| Error::InvalidRequest(format!("unknown document `{name}")))?;

        Ok(Some(self.make_hints(source)))
    }

    /// `shutdown`
    fn shutdown(&mut self) {}
}
