use crossbeam::channel::Sender;
use lsp_server::{Message, Notification};
use lsp_types as lsp;
use lsp_types::notification;

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

    pub fn message(&mut self, typ: lsp::MessageType, message: impl Into<String>) {
        self.notify::<notification::ShowMessage>(lsp::ShowMessageParams {
            typ,
            message: message.into(),
        });
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

    fn notify<N: notification::Notification>(&mut self, params: N::Params) {
        self.messages
            .send(Message::Notification(Notification::new(N::METHOD.into(), params)))
            .expect("attempting to notify on closed channel");
    }
}
