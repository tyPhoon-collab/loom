pub mod check;
pub mod fmt;
pub mod live;
pub mod parse;
pub mod play;
pub mod ports;
pub mod save;
pub mod studio;

use crate::cli::Commands;
use loom::config::{load_global_config, GlobalConfig};
use miette::Result;

pub fn run(command: Commands) -> Result<()> {
    let loaded_config = load_global_config();
    let config = &loaded_config.config;

    match command {
        Commands::Check { input } => check::handle_check(input),
        Commands::Parse {
            input,
            format,
            sort,
            filter,
            summary,
        } => parse::handle_parse(input, format, sort, &filter, summary),
        Commands::Play { input, port } => {
            let port = resolve_midi_port(port, config);
            play::handle_play(input, port)
        }
        Commands::Live { input, port } => {
            let port = resolve_midi_port(port, config);
            live::handle_live(input, port, loaded_config.status_message())
        }
        Commands::Studio { input, port } => {
            let port = resolve_midi_port(port, config);
            studio::handle_studio(input, port, loaded_config.status_message())
        }
        Commands::Save { input, output } => save::handle_save(input, output),
        Commands::Fmt { input, check } => fmt::handle_fmt(input, check),
        Commands::Ports => ports::handle_ports(),
    }
}

fn resolve_midi_port(cli_port: Option<usize>, config: &GlobalConfig) -> usize {
    cli_port.or(config.midi.output_port).unwrap_or(0)
}
