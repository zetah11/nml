#![feature(if_let_guard, lint_reasons)]

mod args;
mod batch;
mod frontend;
mod lsp;
mod meta;
mod modules;

use std::process::ExitCode;

use self::args::{Args, Check, Command, Lsp};
use self::batch::BatchError;
use self::lsp::LspError;

fn main() -> ExitCode {
    let args: Args = argh::from_env();

    match args.command {
        Command::Lsp(Lsp { log, stdio: true }) => {
            log::set_max_level(log.to_level_filter());
            lsp_error(lsp::run())
        }

        Command::Lsp(_) => lsp_error(Err(LspError::NoChannel)),

        Command::Check(Check { path, log }) => {
            if let Some(log) = log {
                if let Some(level) = log.to_level_filter().to_level() {
                    simple_logger::init_with_level(level).expect("this is the only logger");
                }
            } else if std::env::var("RUST_LOG").is_ok() {
                simple_logger::init_with_env().expect("this is the only logger");
            }

            batch_error(batch::run(&path))
        }
    }
}

fn batch_error(result: Result<(), BatchError>) -> ExitCode {
    match result {
        Ok(()) => return ExitCode::SUCCESS,

        Err(BatchError::IoError(err)) => {
            eprintln!("io error: {err}");
        }

        Err(BatchError::CompilerError {
            num_errors,
            num_warnings,
        }) => {
            let es = if num_errors != 1 { "s" } else { "" };
            let ws = if num_warnings != 1 { "s" } else { "" };
            match (num_errors, num_warnings) {
                (0, _) => {
                    eprintln!("finished with {num_warnings} warning{ws}");
                    return ExitCode::SUCCESS;
                }

                (_, 0) => {
                    eprintln!("finished with {num_errors} error{es}");
                }

                _ => {
                    eprintln!(
                        "finished with {num_errors} error{es} and {num_warnings} warning{ws}"
                    );
                }
            }
        }
    }

    ExitCode::FAILURE
}

fn lsp_error(result: Result<(), LspError>) -> ExitCode {
    match result {
        Ok(()) => return ExitCode::SUCCESS,

        Err(LspError::ImproperExit) => {
            eprintln!("client sent improper exit order");
        }

        Err(LspError::ExtractNotificationError(err)) => {
            eprintln!("error parsing notification: {err}");
        }

        Err(LspError::ExtractRequestError(err)) => {
            eprintln!("error parsing request: {err}");
        }

        Err(LspError::ProtocolError(err)) => {
            eprintln!("protocol error: {err}");
        }

        Err(LspError::JsonError(err)) => {
            eprintln!("json error: {err}");
        }

        Err(LspError::NoChannel) => {
            eprintln!("no communication channel specified");
        }
    }

    ExitCode::FAILURE
}
