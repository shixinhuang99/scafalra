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

	/// Interactive mode
	#[arg(short, long, global = true)]
	pub interactive: bool,
}

#[derive(Subcommand)]
pub enum Command {
	/// List all templates
	List(ListArgs),

	/// Remove specified templates
	Remove(RemoveArgs),

	/// Rename a template
	Rename(RenameArgs),

	/// Add template from GitHub repository
	Add(AddArgs),

	/// Copy the template folder to the specified directory
	Create(CreateArgs),

	/// Configure or display your GitHub personal access token
	Token(TokenArgs),
}

#[derive(Args, Debug)]
pub struct ListArgs {
	/// Output in table format
	#[arg(short, long)]
	pub table: bool,
}

#[derive(Args, Debug)]
pub struct RemoveArgs {
	/// Template name List
	pub names: Option<Vec<String>>,
}

#[derive(Args, Debug)]
pub struct RenameArgs {
	/// Template name
	pub name: Option<String>,

	/// New Template name
	pub new_name: Option<String>,
}

#[derive(Args, Debug)]
#[cfg_attr(test, derive(Clone))]
pub struct AddArgs {
	/// owner/name/.../subdir
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

	/// Specify template name, if a subdir is provided, the last level of the
	/// subdir will be used as the name
	#[arg(long)]
	pub name: Option<String>,

	/// Specify sub directory
	#[arg(long)]
	pub subdir: Option<String>,

	/// Specify branch
	#[arg(long)]
	pub branch: Option<String>,

	/// Specify tag
	#[arg(long)]
	pub tag: Option<String>,

	/// Specify commit
	#[arg(long)]
	pub commit: Option<String>,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
	/// Template name
	pub name: Option<String>,

	/// Specified destination(defaults to the current directory)
	#[arg(short, long)]
	pub destination: Option<PathBuf>,

	#[arg(short, long)]
	pub sub_templates: Option<Vec<String>>,
}

#[derive(Args, Debug)]
pub struct TokenArgs {
	pub token: Option<String>,
}

#[cfg(test)]
pub mod test_utils {
	use super::AddArgs;

	pub struct AddArgsMock {
		args: AddArgs,
	}

	impl AddArgsMock {
		pub fn new() -> Self {
			Self {
				args: AddArgs {
					repository: "foo/bar".to_string(),
					depth: 0,
					name: None,
					subdir: None,
					branch: None,
					tag: None,
					commit: None,
				},
			}
		}

		pub fn build(&self) -> AddArgs {
			self.args.clone()
		}

		pub fn depth(&mut self, depth: u8) -> &mut Self {
			self.args.depth = depth;

			self
		}

		pub fn name(&mut self, name: &str) -> &mut Self {
			self.args.name = Some(name.to_string());

			self
		}

		pub fn subdir(&mut self, subdir: &str) -> &mut Self {
			self.args.subdir = Some(subdir.to_string());

			self
		}
	}
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
