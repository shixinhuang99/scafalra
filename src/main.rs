mod cli;
mod github_api;
mod repotitory;

fn main() {
    run();
}

fn run() {
    use clap::Parser;
    use cli::{Cli, Command};

    let cli = Cli::parse();

    println!("verbose: {}", cli.verbose);

    if let Some(token) = cli.token {
        println!("specify token: {}", token);
    }

    match cli.command {
        Command::LIST(args) => {
            println!("list args: {:?}", args);
        }
        Command::REMOVE(args) => {
            println!("remove args: {:?}", args);
        }
        Command::MV(args) => {
            println!("mv args: {:?}", args);
        }
        Command::ADD(args) => {
            println!("add args: {:?}", args);
        }
        Command::CREATE(args) => {
            println!("create args: {:?}", args);
        }
        Command::TOKEN(args) => {
            println!("token args: {:?}", args);
        }
    }
}
