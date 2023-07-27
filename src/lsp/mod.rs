mod tokens;

use dashmap::mapref::entry::Entry;
use dashmap::DashMap;
use nml_compiler::source::{Source, Sources};
use tower_lsp::jsonrpc::{Error, Result};
use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidOpenTextDocumentParams, InitializeParams, InitializeResult,
    SemanticTokensFullOptions, SemanticTokensOptions, SemanticTokensParams, SemanticTokensResult,
    SemanticTokensServerCapabilities, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
    TextDocumentSyncKind, Url,
};
use tower_lsp::{Client, LanguageServer, LspService};

use crate::meta;

pub async fn run() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Server::new);
    tower_lsp::Server::new(stdin, stdout, socket).serve(service).await;
}

struct Server {
    #[allow(unused)]
    client: Client,

    tracked: DashMap<Url, Source>,
    sources: Sources,
}

impl Server {
    fn new(client: Client) -> Self {
        Self { client, tracked: DashMap::new(), sources: Sources::new() }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Server {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        let result = InitializeResult {
            server_info: Some(ServerInfo {
                name: meta::NAME.into(),
                version: Some(meta::VERSION.into()),
            }),

            capabilities: ServerCapabilities {
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: tokens::legend::get(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            ..Default::default()
                        },
                    ),
                ),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
        };

        Ok(result)
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let name = params.text_document.uri;
        let data = params.text_document.text;

        match self.tracked.entry(name) {
            Entry::Occupied(mut entry) => entry.get_mut().content = data,
            Entry::Vacant(entry) => {
                entry.insert(self.sources.add(data));
            }
        }
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        let name = params.text_document.uri;
        assert_eq!(1, params.content_changes.len(), "full synchronization");

        let data = params.content_changes.remove(0).text;

        match self.tracked.entry(name) {
            Entry::Occupied(mut entry) => entry.get_mut().content = data,
            Entry::Vacant(entry) => {
                entry.insert(self.sources.add(data));
            }
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let document = {
            let name = params.text_document.uri;
            self.tracked.get(&name).ok_or_else(Error::invalid_request)?
        };

        Ok(Some(SemanticTokensResult::Tokens(self.compute_tokens(&document))))
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
