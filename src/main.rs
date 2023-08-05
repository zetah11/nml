use anyhow::anyhow;
use nmlc::args::{Args, Check, Command, Lsp};

fn main() -> anyhow::Result<()> {
    let args: Args = argh::from_env();

    match args.command {
        Command::Lsp(Lsp { log, stdio: true }) => {
            log::set_max_level(log.to_level_filter());
            nmlc::lsp::run()
        }

        Command::Lsp(_) => Err(anyhow!("no communication channel specified")),

        Command::Check(Check { path }) => {
            simple_logger::init_with_env()?;
            nmlc::batch::run(&path)
        }
    }
}
