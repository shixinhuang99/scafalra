use std::path::PathBuf;

use clap::{
	builder::{PossibleValuesParser, TypedValueParser as _},
	Args, Parser, Subcommand,
};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
	#[command(subcommand)]
	pub command: Option<Command>,

	/// Use debug output
	#[arg(long, global = true)]
	pub debug: bool,

	/// Specify the GitHub personal access token
	#[arg(long, global = true)]
	pub token: Option<String>,

	/// Display of scafalra's data storage location
	#[arg(long)]
	pub proj_dir: bool,
}

#[derive(Subcommand)]
pub enum Command {
	/// List all scaffolds
	List(ListArgs),

	/// Remove specified scaffolds
	Remove(RemoveArgs),

	/// Rename a scaffold
	Mv(MvArgs),

	/// Add scaffolds from GitHub repository
	Add(AddArgs),

	/// Copy the scaffold folder to the specified directory
	Create(CreateArgs),

	/// Configure or display your GitHub personal access token
	Token(TokenArgs),

	/// Update self
	#[cfg(feature = "self_update")]
	Update(UpdateArgs),

	/// Uninstall self
	#[cfg(feature = "self_update")]
	Uninstall(UninstallArgs),
}

#[derive(Args, Debug)]
pub struct ListArgs {
	/// Output in table format
	#[arg(short, long)]
	pub table: bool,
}

#[derive(Args, Debug)]
pub struct RemoveArgs {
	pub names: Vec<String>,
}

#[derive(Args, Debug)]
pub struct MvArgs {
	pub name: String,
	pub new_name: String,
}

#[derive(Args, Debug)]
pub struct AddArgs {
	/// owner/name/.../subdir?(branch|tag|commit)=...
	pub repository: String,

	/// The depth to go when recursing repository
	#[arg(
        short,
        long,
        default_value_t = 0,
        value_parser = PossibleValuesParser::new(["0", "1"])
            .map(|s| s.parse::<u8>().unwrap())
    )]
	pub depth: u8,

	/// Specify scaffold name, if a subdir is provided, the last level of the
	/// subdir will be used as the name
	#[arg(long)]
	pub name: Option<String>,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
	/// Scaffold name
	pub name: String,

	/// Specified directory(defaults to the current directory)
	pub directory: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct TokenArgs {
	pub token: Option<String>,
}

#[cfg(feature = "self_update")]
#[derive(Args, Debug)]
pub struct UpdateArgs {
	/// Check for updates to self
	#[arg(long)]
	pub check: bool,
}

#[cfg(feature = "self_update")]
#[derive(Args, Debug)]
pub struct UninstallArgs {
	/// Keep the data
	#[arg(long)]
	pub keep_data: bool,
}

#[cfg(test)]
mod tests {
	use clap::CommandFactory;

	use super::Cli;

	#[test]
	fn verify_cli() {
		Cli::command().debug_assert();
	}
}
