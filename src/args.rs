use std::path::PathBuf;
use std::str::FromStr;

use argh::FromArgs;

/// test message 123
#[derive(FromArgs, Debug)]
pub struct Args {
    #[argh(subcommand)]
    pub command: Command,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
pub enum Command {
    Lsp(Lsp),
    Check(Check),
}

/// Check the package for static errors.
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "check")]
pub struct Check {
    /// the source file to check
    #[argh(positional)]
    pub path: PathBuf,

    /// the amount of logging to perform
    #[argh(option)]
    pub log: Option<LogLevel>,
}
/// Run the compiler as a language server.
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "lsp")]
pub struct Lsp {
    /// the amount of logging to perform.
    #[argh(option, default = "LogLevel::Off")]
    pub log: LogLevel,

    /// use stdio as the communication channel
    #[argh(switch)]
    pub stdio: bool,
}

#[derive(Clone, Copy, Debug)]
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

impl FromStr for LogLevel {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "off" => Ok(Self::Off),
            "trace" => Ok(Self::Trace),
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "warning" => Ok(Self::Warning),
            "error" => Ok(Self::Error),

            _ => Err("expected one of `off`, `trace`, `debug`, `info`, `warning`, or `error`"),
        }
    }
}
