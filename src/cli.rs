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
	/// List all templates
	List(ListArgs),

	/// Remove specified templates
	Remove(RemoveArgs),

	/// Rename a template
	Mv(MvArgs),

	/// Add template from GitHub repository
	Add(AddArgs),

	/// Copy the template folder to the specified directory
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
	pub name: String,

	/// Specified directory(defaults to the current directory)
	pub directory: Option<PathBuf>,

	/// Comma-separated list of glob
	#[arg(long)]
	pub with: Option<String>,
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
pub mod test_utils {
	use super::AddArgs;

	pub struct AddArgsMock {
		args: AddArgs,
	}

	impl AddArgsMock {
		pub fn new() -> Self {
			Self {
				args: AddArgs {
					repository: "".to_string(),
					depth: 0,
					name: None,
					branch: None,
					tag: None,
					commit: None,
				},
			}
		}

		pub fn build(&self) -> AddArgs {
			self.args.clone()
		}

		pub fn repository(&mut self, repository: &str) -> &mut Self {
			self.args.repository = repository.to_string();

			self
		}

		pub fn depth(&mut self, depth: u8) -> &mut Self {
			self.args.depth = depth;

			self
		}

		pub fn name(&mut self, name: &str) -> &mut Self {
			self.args.name = Some(name.to_string());

			self
		}

		pub fn branch(&mut self, branch: &str) -> &mut Self {
			self.args.branch = Some(branch.to_string());

			self
		}

		pub fn tag(&mut self, tag: &str) -> &mut Self {
			self.args.tag = Some(tag.to_string());

			self
		}

		pub fn commit(&mut self, commit: &str) -> &mut Self {
			self.args.commit = Some(commit.to_string());

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
