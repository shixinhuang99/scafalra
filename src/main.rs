mod cli;
mod colorize;
mod config;
mod debug;
mod github_api;
mod repository;
mod scafalra;
mod store;
mod utils;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use debug::set_debug;
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

	if cli.debug {
		set_debug(cli.debug);
	}

	let mut scafalra = Scafalra::new(&home_dir, None, cli.token.as_deref())?;

	if cli.root_dir {
		println!("{}", scafalra.root_dir.display());
		return Ok(());
	}

	if let Some(command) = cli.command {
		match command {
			Command::List(args) => scafalra.list(args),
			Command::Remove(args) => scafalra.remove(args)?,
			Command::Mv(args) => scafalra.mv(args)?,
			Command::Add(args) => scafalra.add(args)?,
			Command::Create(args) => scafalra.create(args)?,
			Command::Token(args) => scafalra.config_or_display_token(args)?,
		}
	}

	Ok(())
}
