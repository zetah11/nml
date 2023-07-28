use log::LevelFilter;
use log::{Level, Log};
use tokio::runtime::Handle;
use tower_lsp::lsp_types as lsp;
use tower_lsp::Client;

pub struct Logger {
    client: Client,
    min: Level,
    rt: Handle,
}

impl Logger {
    pub fn init(level: LevelFilter, client: Client) {
        log::set_max_level(level);
        let Some(level) = level.to_level() else { return; };
        let logger = Self { client, min: level, rt: Handle::current() };
        let log: Box<dyn Log> = Box::new(logger);
        log::set_logger(Box::leak(log)).expect("logger is only installed once");
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        panic!("{} <= {} = {}", metadata.level(), self.min, metadata.level() <= self.min);
    }

    fn log(&self, record: &log::Record) {
        let typ = match record.level() {
            Level::Error => lsp::MessageType::ERROR,
            Level::Warn => lsp::MessageType::WARNING,
            Level::Info => lsp::MessageType::INFO,
            Level::Debug | Level::Trace => lsp::MessageType::LOG,
        };

        let prefix = record.module_path().map(|s| format!("{s}: ")).unwrap_or_default();
        let message = format!("{prefix}{}", record.args());

        let client = self.client.clone();
        self.rt.spawn(async move { client.log_message(typ, message).await });
    }

    fn flush(&self) {
        todo!()
    }
}
