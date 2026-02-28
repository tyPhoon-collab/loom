pub mod check;
pub mod fmt;
pub mod live;
pub mod parse;
pub mod play;
pub mod ports;
pub mod save;

use crate::cli::Commands;
use miette::Result;

pub fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Check { input } => check::handle_check(input),
        Commands::Parse {
            input,
            format,
            sort,
            filter,
            summary,
        } => parse::handle_parse(input, format, sort, &filter, summary),
        Commands::Play { input, port } => play::handle_play(input, port),
        Commands::Live { input, port } => live::handle_live(input, port),
        Commands::Save { input, output } => save::handle_save(input, output),
        Commands::Fmt { input, check } => fmt::handle_fmt(input, check),
        Commands::Ports => ports::handle_ports(),
    }
}
