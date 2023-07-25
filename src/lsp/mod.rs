use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    InitializeParams, InitializeResult, InitializedParams, MessageType, ServerCapabilities,
    ServerInfo,
};
use tower_lsp::{Client, LanguageServer, LspService};

use crate::meta;

pub async fn run() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Server);
    tower_lsp::Server::new(stdin, stdout, socket).serve(service).await;
}

struct Server(Client);

#[tower_lsp::async_trait]
impl LanguageServer for Server {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        let result = InitializeResult {
            server_info: Some(ServerInfo {
                name: meta::NAME.into(),
                version: Some(meta::VERSION.into()),
            }),

            capabilities: ServerCapabilities { ..Default::default() },
        };

        Ok(result)
    }

    async fn initialized(&self, _: InitializedParams) {
        self.0.log_message(MessageType::INFO, "initialized!").await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
