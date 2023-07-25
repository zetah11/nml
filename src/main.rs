use clap::Parser;
use soml::args::{Args, Channel, Command};

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.command {
        Command::Lsp { channel: Channel::Stdio } => soml::lsp::run().await,
    }
}
