use clap::Parser;
use nmlc::args::{Args, Channel, Command};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Lsp { channel: Channel { stdio: true }, log } => {
            log::set_max_level(log.to_level_filter());
            nmlc::lsp::run().await
        }

        Command::Lsp { .. } => unreachable!(),
    }
}
