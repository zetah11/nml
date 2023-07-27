use clap::{Parser, Subcommand};

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
    },
}

#[derive(Debug, clap::Args)]
#[group(required = true, multiple = false)]
pub struct Channel {
    #[arg(long)]
    pub stdio: bool,
}
