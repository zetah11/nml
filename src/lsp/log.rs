use log::{Level, Log};
use lsp_types as lsp;

use super::framework::Client;

pub struct Logger {
    client: Client,
}

impl Logger {
    pub fn init(client: Client) {
        let logger = Self { client };
        let log: Box<dyn Log> = Box::new(logger);
        log::set_logger(Box::leak(log)).expect("logger is only installed once");
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level().to_level_filter() <= log::max_level()
    }

    fn log(&self, record: &log::Record) {
        // Ignore lsp_server debug messages
        if record.module_path().map(|path| path.starts_with("lsp_server")).unwrap_or(false) {
            return;
        }

        if self.enabled(record.metadata()) {
            let typ = match record.level() {
                Level::Error => lsp::MessageType::ERROR,
                Level::Warn => lsp::MessageType::WARNING,
                Level::Info => lsp::MessageType::INFO,
                Level::Debug | Level::Trace => lsp::MessageType::LOG,
            };

            let prefix = record.module_path().map(|s| format!("{s}: ")).unwrap_or_default();
            let message = format!("{prefix}{}", record.args());

            self.client.clone().log(typ, message);
        }
    }

    fn flush(&self) {
        todo!()
    }
}
