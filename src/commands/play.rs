use loom::dsl::parser;
use loom::{compiler, player};
use miette::{miette, Result};
use std::path::PathBuf;

pub fn handle_play(input: PathBuf, port: usize) -> Result<()> {
    let song = parser::parse_song_from_path(&input)?;
    let compiler_inst = compiler::Compiler::new(&song)?;
    let events = compiler_inst
        .compile(&song)
        .map_err(|e| miette!("Compiler error: {}", e))?;

    let mut player_inst = player::Player::new(port)?;
    player_inst.play(&events, &song.metadata)?;
    Ok(())
}
