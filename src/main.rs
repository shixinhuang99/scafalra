mod api;
mod cli;
mod colorize;
mod config;
mod debug;
mod interactive;
mod json;
mod path_ext;
mod repository;
mod scafalra;
mod store;
mod sub_template;
mod template;

use std::env;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use debug::trun_on_debug;
use directories::ProjectDirs;
use scafalra::Scafalra;

fn main() {
	if let Err(err) = run() {
		eprintln!("{}", err);
	}
}

fn run() -> Result<()> {
	let cli = Cli::parse();

	if cli.debug || env::var("SCAFALRA_DEBUG").is_ok() {
		trun_on_debug();
	}

	let scfalra_dir = if cfg!(feature = "_try") {
		std::path::PathBuf::from("tmp/sca-test")
	} else {
		ProjectDirs::from("", "", "scafalra")
			.ok_or(anyhow::anyhow!(
				"Failed to create scafalra project directory"
			))?
			.config_dir()
			.to_path_buf()
	};

	let mut scafalra = Scafalra::new(scfalra_dir, None, cli.token.as_deref())?;

	if cli.proj_dir {
		println!("{}", scafalra.path.to_string_lossy());
		return Ok(());
	}

	if cli.interactive {
		scafalra.interactive_mode = true;
	}

	if let Some(command) = cli.command {
		match command {
			Command::List(args) => scafalra.list(args),
			Command::Remove(args) => scafalra.remove(args)?,
			Command::Rename(args) => scafalra.rename(args)?,
			Command::Add(args) => scafalra.add(args)?,
			Command::Create(args) => scafalra.create(args)?,
			Command::Token(args) => scafalra.token(args)?,
		}
	}

	Ok(())
}
