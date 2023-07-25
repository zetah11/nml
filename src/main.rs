use clap::Parser;
use nmlc::args::{Args, Channel, Command};

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.command {
        Command::Lsp { channel: Channel::Stdio } => nmlc::lsp::run().await,
    }
}
