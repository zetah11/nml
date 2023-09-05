use lsp_server::{ExtractError, Notification, ProtocolError, Request};
use serde_json::Error;

pub enum LspError {
    ImproperExit,
    ExtractNotificationError(ExtractError<Notification>),
    ExtractRequestError(ExtractError<Request>),
    ProtocolError(ProtocolError),
    JsonError(Error),
    NoChannel,
}

impl From<ExtractError<Notification>> for LspError {
    fn from(value: ExtractError<Notification>) -> Self {
        Self::ExtractNotificationError(value)
    }
}

impl From<ExtractError<Request>> for LspError {
    fn from(value: ExtractError<Request>) -> Self {
        Self::ExtractRequestError(value)
    }
}

impl From<ProtocolError> for LspError {
    fn from(value: ProtocolError) -> Self {
        Self::ProtocolError(value)
    }
}

impl From<Error> for LspError {
    fn from(value: Error) -> Self {
        Self::JsonError(value)
    }
}
