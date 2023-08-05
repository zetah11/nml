use clap::Parser;
use nmlc::args::{Args, Channel, Command};

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Lsp { channel: Channel { stdio: true }, log } => {
            log::set_max_level(log.to_level_filter());
            nmlc::lsp::run()
        }

        Command::Lsp { .. } => unreachable!(),

        Command::Check { path } => {
            pretty_env_logger::init();
            nmlc::batch::run(&path)
        }
    }
}
