mod cli;
mod config;
mod github_api;
mod macros;
mod repository;
mod scafalra;
mod store;
mod utils;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use scafalra::Scafalra;

fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
    }
}

fn run() -> Result<()> {
    let Some(home_dir) = home::home_dir() else {
        anyhow::bail!("Impossible to get your home dir");
    };

    let cli = Cli::parse();

    let mut scafalra =
        Scafalra::new(&home_dir, None, cli.token.as_ref().map(|v| v.as_str()))?;

    match cli.command {
        Command::LIST(args) => scafalra.list(args),
        Command::REMOVE(args) => scafalra.remove(args)?,
        Command::MV(args) => scafalra.mv(args)?,
        Command::ADD(args) => scafalra.add(args)?,
        Command::CREATE(args) => scafalra.create(args)?,
        Command::TOKEN(args) => scafalra.config_or_display_token(args)?,
    }

    Ok(())
}
