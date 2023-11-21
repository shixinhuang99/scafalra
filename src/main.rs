mod cli;
mod colorize;
mod config;
mod debug;
mod github_api;
mod json_content;
mod repository;
mod scafalra;
mod store;
mod toml_content;
mod utf8_path;
mod utils;

use std::env;

use anyhow::Result;
use camino::Utf8PathBuf;
use clap::Parser;
use cli::{Cli, Command};
use debug::trun_on_debug;
use scafalra::Scafalra;

fn main() {
	if let Err(err) = try_main() {
		eprintln!("{:?}", err);
	}
}

fn try_main() -> Result<()> {
	let home_dir = Utf8PathBuf::from_path_buf(
		home::home_dir()
			.ok_or(anyhow::anyhow!("Impossible to get your home directory"))?,
	)
	.map_err(|err_path| {
		anyhow::anyhow!(
			"Home directory `{}` it is not valid UTF-8 path",
			err_path.display()
		)
	})?;

	let cli = Cli::parse();

	if cli.debug || env::var("DEBUG_LOG").is_ok() {
		trun_on_debug();
	}

	let mut scafalra = Scafalra::new(&home_dir, None, cli.token.as_deref())?;

	if cli.root_dir {
		println!("{}", scafalra.root_dir);
		return Ok(());
	}

	if let Some(command) = cli.command {
		match command {
			Command::List(args) => scafalra.list(args),
			Command::Remove(args) => scafalra.remove(args)?,
			Command::Mv(args) => scafalra.mv(args)?,
			Command::Add(args) => scafalra.add(args)?,
			Command::Create(args) => scafalra.create(args)?,
			Command::Token(args) => scafalra.set_or_display_token(args)?,
			Command::Update(args) => scafalra.update(args)?,
			Command::Uninstall(args) => scafalra.uninstall(args)?,
		}
	}

	Ok(())
}
