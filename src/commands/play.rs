use loom::dsl::parser;
use loom::{compiler, player};
use miette::{miette, IntoDiagnostic, Result};
use std::fs;
use std::path::PathBuf;

pub fn handle_play(input: PathBuf, port: usize) -> Result<()> {
    let content = fs::read_to_string(&input).into_diagnostic()?;
    let song = parser::parse_song(content)?;
    let compiler_inst = compiler::Compiler::new(&song)?;
    let (note_events, control_events) = compiler_inst
        .compile_with_controls(&song)
        .map_err(|e| miette!("Compiler error: {}", e))?;

    let mut player_inst = player::Player::new(port)?;
    player_inst.play(&note_events, &control_events, &song.metadata)?;
    Ok(())
}
