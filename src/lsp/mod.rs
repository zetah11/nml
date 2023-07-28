mod check;
mod diagnostics;
mod log;
mod sync;
mod tokens;

use dashmap::{DashMap, DashSet};
use nml_compiler::intern::ThreadedRodeo;
use nml_compiler::names::Ident;
use nml_compiler::source::{Source, SourceId, Sources};
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::{self as lsp, Url};
use tower_lsp::{Client, LanguageServer, LspService};

use self::log::Logger;
use crate::meta;

pub async fn run(log: ::log::LevelFilter) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| {
        Logger::init(log, client.clone());
        Server::new(client)
    });

    tower_lsp::Server::new(stdin, stdout, socket).serve(service).await;
}

struct Server {
    #[allow(unused)]
    client: Client,

    tracked: DashMap<Url, Source>,
    names: DashMap<SourceId, Url>,
    sources: Sources,

    idents: ThreadedRodeo<Ident>,
    errors: DashSet<Url>,
}

impl Server {
    fn new(client: Client) -> Self {
        Self {
            client,
            tracked: DashMap::new(),
            names: DashMap::new(),
            sources: Sources::new(),
            idents: ThreadedRodeo::new(),
            errors: DashSet::new(),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Server {
    async fn initialize(&self, _: lsp::InitializeParams) -> Result<lsp::InitializeResult> {
        let result = lsp::InitializeResult {
            server_info: Some(lsp::ServerInfo {
                name: meta::NAME.into(),
                version: Some(meta::VERSION.into()),
            }),

            capabilities: lsp::ServerCapabilities {
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
            },
        };

        Ok(result)
    }

    async fn initialized(&self, _: lsp::InitializedParams) {
        self.client.log_message(lsp::MessageType::INFO, "nmlc lsp initialized").await;
    }

    async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
        let name = params.text_document.uri;
        let text = params.text_document.text;
        self.insert_document(name.clone(), text);

        let source = self.tracked.get(&name).expect("just inserted");
        self.check_source(&source).await;
    }

    async fn did_change(&self, mut params: lsp::DidChangeTextDocumentParams) {
        let name = params.text_document.uri;
        assert_eq!(1, params.content_changes.len(), "full synchronization");

        let text = params.content_changes.remove(0).text;
        self.insert_document(name.clone(), text);

        let source = self.tracked.get(&name).expect("just inserted");
        self.check_source(&source).await;
    }

    async fn semantic_tokens_full(
        &self,
        params: lsp::SemanticTokensParams,
    ) -> Result<Option<lsp::SemanticTokensResult>> {
        let document = {
            let name = params.text_document.uri;
            self.tracked.get(&name).ok_or_else(Error::invalid_request)?
        };

        Ok(Some(lsp::SemanticTokensResult::Tokens(self.compute_tokens(&document))))
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
