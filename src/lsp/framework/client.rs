use crossbeam::channel::Sender;
use lsp_server::{ErrorCode, Message, Notification, RequestId, Response};
use lsp_types as lsp;
use lsp_types::notification;
use serde::Serialize;

use super::Error;

#[derive(Clone, Debug)]
pub struct Client {
    messages: Sender<Message>,
}

impl Client {
    pub(super) fn new(messages: Sender<Message>) -> Self {
        Self { messages }
    }

    pub fn log(&mut self, typ: lsp::MessageType, message: impl Into<String>) {
        self.notify::<notification::LogMessage>(lsp::LogMessageParams {
            typ,
            message: message.into(),
        });
    }

    pub fn log_trace(&mut self, verbose: Option<impl Into<String>>, message: impl Into<String>) {
        self.notify::<notification::LogTrace>(lsp::LogTraceParams {
            message: message.into(),
            verbose: verbose.map(Into::into),
        })
    }

    pub fn publish_diagnostics(
        &mut self,
        uri: lsp::Url,
        diagnostics: Vec<lsp::Diagnostic>,
        version: Option<i32>,
    ) {
        self.notify::<notification::PublishDiagnostics>(lsp::PublishDiagnosticsParams {
            uri,
            diagnostics,
            version,
        });
    }

    pub(super) fn respond(&mut self, id: RequestId, result: Result<impl Serialize, Error>) {
        let e = match result {
            Ok(data) => self.messages.send(Message::Response(Response::new_ok(id, data))),

            Err(e) => {
                let (code, message) = match e {
                    Error::InvalidRequest(message) => (ErrorCode::InvalidRequest, message),
                    Error::InternalError(message) => (ErrorCode::InternalError, message),
                };

                self.messages.send(Message::Response(Response::new_err(id, code as i32, message)))
            }
        };

        e.expect("attempted to send response over closed channel");
    }

    pub(super) fn notify<N: notification::Notification>(&mut self, params: N::Params) {
        self.messages
            .send(Message::Notification(Notification::new(N::METHOD.into(), params)))
            .expect("attempting to notify on closed channel");
    }
}
