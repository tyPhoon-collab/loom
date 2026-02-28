mod cli;
mod commands;

use clap::Parser;
use cli::Cli;
use miette::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();
    commands::run(cli.command)
}
