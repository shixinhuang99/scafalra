mod cli;
mod colorize;
mod config;
mod debug;
mod github_api;
mod json;
mod repository;
mod scafalra;
mod store;
mod utf8_path;
mod utils;

use std::env;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use debug::trun_on_debug;
use scafalra::Scafalra;
use utf8_path::Utf8PathBufExt;

fn main() {
	if let Err(err) = try_main() {
		eprintln!("{:?}", err);
	}
}

fn try_main() -> Result<()> {
	let cli = Cli::parse();

	if cli.debug || env::var("DEBUG_LOG").is_ok() {
		trun_on_debug();
	}

	let proj_dir = directories::ProjectDirs::from("", "", "scafalra")
		.ok_or(anyhow::anyhow!(
			"No valid home directory path could be retrieved from your \
			 operating system"
		))?
		.config_dir()
		.into_utf8_path_buf()?;

	let mut scafalra = Scafalra::new(proj_dir, None, cli.token.as_deref())?;

	if cli.proj_dir {
		println!("{}", scafalra.proj_dir);
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
			#[cfg(feature = "self_update")]
			Command::Update(args) => scafalra.update(args)?,
			#[cfg(feature = "self_update")]
			Command::Uninstall(args) => scafalra.uninstall(args)?,
		}
	}

	Ok(())
}
