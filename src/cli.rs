use clap::{
    builder::{PossibleValuesParser, TypedValueParser as _},
    Args, Parser, Subcommand,
};

#[derive(Parser)]
#[command(name = "scafalra", author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Use verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Specify the GitHub personal access token(classic)
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

    /// Configure or display your GitHub personal access token(classic)
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
    ///
    /// If a subdir is provided, the last level of the subdir will be used as
    /// the scaffold name
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
    /// Scaffold name
    pub name: String,

    /// Specified directory(defaults to the current directory)
    pub directory: Option<String>,
}

#[derive(Args, Debug)]
pub struct TokenArgs {
    pub token: Option<String>,
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
