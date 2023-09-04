use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use log::{Level, Log};
use lsp::TraceValue;
use lsp_types as lsp;

use super::framework::Client;

#[derive(Debug)]
pub struct AtomicTraceValue {
    value: AtomicU8,
}

impl AtomicTraceValue {
    pub fn new(initial: TraceValue) -> Self {
        let this = Self {
            value: AtomicU8::new(0),
        };
        this.store(initial);
        this
    }

    pub fn load(&self) -> TraceValue {
        match self.value.load(Ordering::SeqCst) {
            0 => TraceValue::Off,
            1 => TraceValue::Messages,
            2 => TraceValue::Verbose,
            _ => unreachable!(),
        }
    }

    pub fn store(&self, value: TraceValue) {
        self.value.store(
            match value {
                TraceValue::Off => 0,
                TraceValue::Messages => 1,
                TraceValue::Verbose => 2,
            },
            Ordering::SeqCst,
        );
    }
}

impl Default for AtomicTraceValue {
    fn default() -> Self {
        Self::new(TraceValue::Off)
    }
}

pub struct Logger {
    client: Client,
    trace: Arc<AtomicTraceValue>,
}

impl Logger {
    pub fn init(trace: Arc<AtomicTraceValue>, client: Client) {
        let logger = Self { client, trace };
        let log: Box<dyn Log> = Box::new(logger);
        log::set_logger(Box::leak(log)).expect("logger is only installed once");
    }
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        match metadata.level() {
            level @ (Level::Trace | Level::Debug) => matches!(
                (level, self.trace.load()),
                (Level::Debug, TraceValue::Messages) | (_, TraceValue::Verbose)
            ),

            other => other.to_level_filter() <= log::max_level(),
        }
    }

    fn log(&self, record: &log::Record) {
        // Ignore lsp_server debug messages
        if record
            .module_path()
            .map(|path| path.starts_with("lsp_server"))
            .unwrap_or(false)
        {
            return;
        }

        if self.enabled(record.metadata()) {
            let prefix = record
                .module_path()
                .map(|s| format!("{s}: "))
                .unwrap_or_default();
            let message = format!("{prefix}{}", record.args());

            let typ = match record.level() {
                Level::Error => lsp::MessageType::ERROR,
                Level::Warn => lsp::MessageType::WARNING,
                Level::Info => lsp::MessageType::INFO,

                Level::Debug | Level::Trace => {
                    self.client.clone().log_trace(None::<String>, message);
                    return;
                }
            };

            self.client.clone().log(typ, message);
        }
    }

    fn flush(&self) {
        todo!()
    }
}
