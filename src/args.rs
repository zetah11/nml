use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Lsp {
        #[arg(
            long,
            value_name = "CHANNEL",
            num_args = 0..=1,
            default_value_t = Channel::Stdio,
            default_missing_value = "stdio",
            value_enum
        )]
        channel: Channel,
    },
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, ValueEnum)]
pub enum Channel {
    Stdio,
}
