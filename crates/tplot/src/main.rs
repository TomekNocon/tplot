mod cli;
use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

fn main() -> Result<()> {
    let args = Cli::parse();
    match args.command {
        Command::Bar(_) => {
            // Wired up in Task 19.
            eprintln!("bar command parsed; pipeline lands in next task");
            Ok(())
        }
        Command::Json => {
            eprintln!("json mode parsed; pipeline lands in next task");
            Ok(())
        }
    }
}
