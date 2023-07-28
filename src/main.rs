use clap::Parser;
use nmlc::args::{Args, Channel, Command};

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.command {
        Command::Lsp { channel: Channel { stdio: true }, log } => {
            nmlc::lsp::run(log.to_level_filter()).await;
        }

        Command::Lsp { .. } => unreachable!(),
    }
}
