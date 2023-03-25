use std::path::PathBuf;

use clap::builder::{PossibleValuesParser, TypedValueParser as _};
use clap::{value_parser, Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "scafalra", author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Use verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Specify the GitHub access token
    #[arg(long)]
    pub token: Option<String>,
}

#[derive(Subcommand)]
pub enum Command {
    /// List all scaffolds
    LIST(ListArgs),

    /// Remove specified scaffolds
    REMOVE(RemoveArgs),

    /// Rename a scaffold
    MV(MvArgs),

    /// Add scaffolds from GitHub repository
    ADD(AddArgs),

    /// Copy the scaffold folder to the specified directory
    CREATE(CreateArgs),

    /// Store your GitHub access token
    TOKEN(TokenArgs),
}

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Output in table format
    #[arg(short, long)]
    pub table: bool,
}

#[derive(Args, Debug)]
pub struct RemoveArgs {
    pub name: Vec<String>,
}

#[derive(Args, Debug)]
pub struct MvArgs {
    pub name: String,
    pub new_name: String,
}

#[derive(Args, Debug)]
pub struct AddArgs {
    pub repository: String,

    /// The depth to go when recursing repository(only support 0 or 1)
    #[arg(
        short,
        long,
        default_value_t = 0,
        value_parser = PossibleValuesParser::new(["0", "1"])
            .map(|s| s.parse::<u8>().unwrap())
    )]
    pub depth: u8,

    /// Specify scaffold name instead of repository name(conflicts with depth)
    #[arg(short, long)]
    pub name: Option<String>,
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    /// scaffold name of GitHub repotistory, Using the repository will only
    /// download and not cache
    pub name_or_repo: String,

    /// Specified directory(defaults to the current directory)
    #[arg(value_parser = value_parser!(PathBuf))]
    pub directory: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct TokenArgs {
    pub token: String,
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
