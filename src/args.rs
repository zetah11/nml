use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Lsp {
        #[command(flatten)]
        channel: Channel,

        #[arg(long, value_enum, default_value_t = LogLevel::Off)]
        log: LogLevel,
    },
}

#[derive(Debug, clap::Args)]
#[group(required = true, multiple = false)]
pub struct Channel {
    #[arg(long)]
    pub stdio: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum LogLevel {
    Off,
    Trace,
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub fn to_level_filter(&self) -> log::LevelFilter {
        match self {
            LogLevel::Off => log::LevelFilter::Off,
            LogLevel::Trace => log::LevelFilter::Trace,
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Warning => log::LevelFilter::Warn,
            LogLevel::Error => log::LevelFilter::Error,
        }
    }
}
