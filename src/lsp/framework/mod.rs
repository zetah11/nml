use std::sync::Arc;

pub use self::client::Client;

mod client;

use crossbeam_channel::Receiver;
use lsp_server::{Connection, Message, Notification, Request};
use lsp_types as lsp;
use lsp_types::notification::{self, Notification as _};
use lsp_types::request::{self, Request as _};

use super::log::AtomicTraceValue;
use super::{LspError, Server};

/// Initialize and run the given server on standard IO.
pub(super) fn stdio(mut builder: impl Builder) -> Result<(), LspError> {
    let (connection, _) = Connection::stdio();
    let (id, params) = connection.initialize_start()?;
    let params: lsp::InitializeParams = serde_json::from_value(params)?;
    let trace = params.trace;
    let result = builder.initialize(params);
    connection.initialize_finish(id, serde_json::to_value(result)?)?;

    let trace = Arc::new(trace.map(AtomicTraceValue::new).unwrap_or_default());

    let client = Client::new(connection.sender.clone());
    let server = builder.build(trace.clone(), client.clone());
    let main = Loop {
        server,
        requests: connection.receiver,
        client,
        state: State::Ready,
        trace,
    };

    match main.run() {
        Final::Exit { properly: true } => Ok(()),
        Final::Exit { properly: false } => Err(LspError::ImproperExit),
        Final::Error(e) => Err(e),
    }
}

pub(super) trait Builder {
    fn build(self, trace: Arc<AtomicTraceValue>, client: Client) -> Server;
    fn initialize(&mut self, params: lsp::InitializeParams) -> lsp::InitializeResult {
        let _ = params;
        Default::default()
    }
}

#[expect(unused)]
#[derive(Clone, Debug)]
pub enum Error {
    InvalidRequest(String),
    InternalError(String),
}

#[derive(Clone, Copy, Debug)]
enum State {
    Ready,
    ShuttingDown,
}

enum Final {
    Exit { properly: bool },
    Error(LspError),
}

impl<E: Into<LspError>> From<E> for Final {
    fn from(value: E) -> Self {
        Self::Error(value.into())
    }
}

struct Loop {
    server: Server,
    requests: Receiver<Message>,
    client: Client,

    state: State,
    trace: Arc<AtomicTraceValue>,
}

impl Loop {
    pub fn run(mut self) -> Final {
        while let Ok(message) = self.requests.recv() {
            let result = match message {
                Message::Notification(notification) => self.handle_notification(notification),
                Message::Request(request) => self.handle_request(request),
                Message::Response(_) => todo!(),
            };

            if let Err(e) = result {
                return e;
            }
        }

        Final::Exit { properly: false }
    }

    fn handle_notification(&mut self, notification: Notification) -> Result<(), Final> {
        match (self.state, notification.method.as_str()) {
            (state, m) if m == notification::Exit::METHOD => {
                return Err(Final::Exit {
                    properly: matches!(state, State::ShuttingDown),
                });
            }

            (State::ShuttingDown, _) => return Err(Final::Exit { properly: false }),

            (_, m) if m == notification::SetTrace::METHOD => {
                let params: lsp::SetTraceParams =
                    notification.extract(notification::SetTrace::METHOD)?;
                self.trace.store(params.value);

                let word = match params.value {
                    lsp::TraceValue::Off => "off",
                    lsp::TraceValue::Messages => "messages",
                    lsp::TraceValue::Verbose => "verbose",
                };

                self.client.log(
                    lsp::MessageType::INFO,
                    format!("set trace amount to `{word}`",),
                );
            }

            (_, m) if m == notification::DidChangeTextDocument::METHOD => {
                let params = notification.extract(notification::DidChangeTextDocument::METHOD)?;
                self.server.did_change_text_document(params);
            }

            (_, m) if m == notification::DidCloseTextDocument::METHOD => {
                let params = notification.extract(notification::DidCloseTextDocument::METHOD)?;
                self.server.did_open_text_document(params);
            }

            (_, m) if m == notification::DidOpenTextDocument::METHOD => {
                let params = notification.extract(notification::DidOpenTextDocument::METHOD)?;
                self.server.did_open_text_document(params);
            }

            (_, m) if m == notification::DidSaveTextDocument::METHOD => {
                let params = notification.extract(notification::DidSaveTextDocument::METHOD)?;
                self.server.did_save_text_document(params);
            }

            (_, m) => {
                self.client.log(
                    lsp::MessageType::ERROR,
                    format!("unexpected notification type `{m}`"),
                );
            }
        }

        Ok(())
    }

    fn handle_request(&mut self, request: Request) -> Result<(), Final> {
        match (self.state, request.method.as_str()) {
            (State::Ready, m) if m == request::Shutdown::METHOD => {
                self.state = State::ShuttingDown;
                self.server.shutdown();
                self.client.respond(request.id, Ok(()));
            }

            (State::ShuttingDown, _) => {
                self.client.respond(
                    request.id,
                    Err::<(), _>(Error::InvalidRequest(
                        "unexpected request after shutdown".into(),
                    )),
                );
            }

            (_, m) if m == request::InlayHintRequest::METHOD => {
                let (id, params) = request.extract(request::InlayHintRequest::METHOD)?;
                let result = self.server.inlay_hints(params);
                self.client.respond(id, result);
            }

            (_, m) if m == request::SemanticTokensFullRequest::METHOD => {
                let (id, params) = request.extract(request::SemanticTokensFullRequest::METHOD)?;
                let result = self.server.semantic_tokens_full(params);
                self.client.respond(id, result);
            }

            (_, m) => {
                self.client.respond(
                    request.id,
                    Err::<(), _>(Error::InvalidRequest(format!("unknown request `{m}`"))),
                );
            }
        }

        Ok(())
    }
}
