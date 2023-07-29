pub use self::client::Client;

mod client;

use crossbeam::channel::{Receiver, Sender};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types as lsp;
use lsp_types::notification::{self, Notification as _};
use lsp_types::request::{self, Request as _};
use serde::Serialize;

use super::Server;

/// Initialize and run the given server on standard IO.
pub(super) fn stdio(mut builder: impl Builder) -> anyhow::Result<()> {
    let (connection, _) = Connection::stdio();
    let (id, params) = connection.initialize_start()?;
    let params = serde_json::from_value(params)?;
    let result = builder.initialize(params);
    connection.initialize_finish(id, serde_json::to_value(result)?)?;

    let client = Client::new(connection.sender.clone());
    let server = builder.build(client);
    let main = Loop {
        server,
        requests: connection.receiver,
        response: connection.sender,
        state: State::Ready,
    };

    match main.run() {
        Final::Exit { properly: true } => Ok(()),
        Final::Exit { properly: false } => Err(anyhow::anyhow!("")),
        Final::Error(e) => Err(e),
    }
}

pub(super) trait Builder {
    fn build(self, client: Client) -> Server;
    fn initialize(&mut self, params: lsp::InitializeParams) -> lsp::InitializeResult {
        let _ = params;
        Default::default()
    }
}

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
    Error(anyhow::Error),
}

impl<E: Into<anyhow::Error>> From<E> for Final {
    fn from(value: E) -> Self {
        Self::Error(value.into())
    }
}

struct Loop {
    server: Server,
    requests: Receiver<Message>,
    response: Sender<Message>,

    state: State,
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
                return Err(Final::Exit { properly: matches!(state, State::ShuttingDown) });
            }

            (State::ShuttingDown, _) => return Err(Final::Exit { properly: false }),

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
                self.response
                    .send(Message::Notification(Notification::new(
                        notification::LogMessage::METHOD.into(),
                        lsp::LogMessageParams {
                            typ: lsp::MessageType::ERROR,
                            message: format!("unexpected notification type `{m}`"),
                        },
                    )))
                    .expect("received unexpected notification");
            }
        }

        Ok(())
    }

    fn handle_request(&mut self, request: Request) -> Result<(), Final> {
        match (self.state, request.method.as_str()) {
            (State::Ready, m) if m == request::Shutdown::METHOD => {
                self.state = State::ShuttingDown;
                self.server.shutdown();
                self.send_response(request.id, Ok(()));
            }

            (State::ShuttingDown, _) => {
                self.send_response(
                    request.id,
                    Err::<(), _>(Error::InvalidRequest("unexpected request after shutdown".into())),
                );
            }

            (_, m) if m == request::SemanticTokensFullRequest::METHOD => {
                let (id, params) = request.extract(request::SemanticTokensFullRequest::METHOD)?;
                let result = self.server.semantic_tokens_full(params);
                self.send_response(id, result);
            }

            (_, m) => {
                self.send_response(
                    request.id,
                    Err::<(), _>(Error::InvalidRequest(format!("unknown request `{m}`"))),
                );
            }
        }

        Ok(())
    }

    fn send_response(&mut self, id: RequestId, result: Result<impl Serialize, Error>) {
        let e = match result {
            Ok(data) => self.response.send(Message::Response(Response::new_ok(id, data))),

            Err(e) => {
                let (code, message) = match e {
                    Error::InvalidRequest(message) => (ErrorCode::InvalidRequest, message),
                    Error::InternalError(message) => (ErrorCode::InternalError, message),
                };

                self.response.send(Message::Response(Response::new_err(id, code as i32, message)))
            }
        };

        e.expect("attempted to send response over closed channel");
    }
}
