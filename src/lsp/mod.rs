mod check;
mod diagnostics;
mod framework;
mod log;
mod sync;
mod tokens;

use lsp_types::{self as lsp, Url};
use nml_compiler::intern::ThreadedRodeo;
use nml_compiler::names::Ident;
use nml_compiler::source::{Source, SourceId, Sources};
use std::collections::{HashMap, HashSet};

use self::framework::{Client, Error};
use self::log::Logger;
use crate::meta;

pub fn run() -> anyhow::Result<()> {
    framework::stdio(Builder::new())
}

struct Builder;

impl Builder {
    fn new() -> Self {
        Self
    }
}

impl framework::Builder for Builder {
    fn build(self, client: Client) -> Server {
        Logger::init(client.clone());
        Server::new(client)
    }

    fn initialize(&mut self, _: lsp::InitializeParams) -> lsp::InitializeResult {
        let server_info =
            Some(lsp::ServerInfo { name: meta::NAME.into(), version: Some(meta::VERSION.into()) });

        let capabilities = lsp::ServerCapabilities {
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

        lsp::InitializeResult { server_info, capabilities }
    }
}

struct Server {
    #[allow(unused)]
    client: Client,

    tracked: HashMap<Url, Source>,
    names: HashMap<SourceId, Url>,
    sources: Sources,

    idents: ThreadedRodeo<Ident>,
    errors: HashSet<Url>,
}

impl Server {
    fn new(client: Client) -> Self {
        Self {
            client,
            tracked: HashMap::new(),
            names: HashMap::new(),
            sources: Sources::new(),
            idents: ThreadedRodeo::new(),
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
            let source = self.tracked.get(&name).expect("just inserted");
            self.check_source(source)
        };

        self.send_diagnostics(&mut errors);
    }

    /// `textDocument/didOpen`
    fn did_open_text_document(&mut self, params: lsp::DidOpenTextDocumentParams) {
        let name = params.text_document.uri;
        let text = params.text_document.text;
        self.insert_document(name.clone(), text);

        let mut errors = {
            let source = self.tracked.get(&name).expect("just inserted");
            self.check_source(source)
        };

        self.send_diagnostics(&mut errors);
    }

    /// `textDocument/didSave`
    fn did_save_text_document(&mut self, _: lsp::DidSaveTextDocumentParams) {
        todo!()
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

        Ok(Some(lsp::SemanticTokensResult::Tokens(self.compute_tokens(document))))
    }

    /// `shutdown`
    fn shutdown(&mut self) {}
}
