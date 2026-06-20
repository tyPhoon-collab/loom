use loom::compiler;
use loom::dsl::parser;
use loom::midi::file;
use miette::{miette, Result};
use std::path::PathBuf;

pub fn handle_save(input: PathBuf, output: Option<PathBuf>) -> Result<()> {
    let song = parser::parse_song_from_path(&input)?;
    let compiler_inst = compiler::Compiler::new(&song)?;
    let events = compiler_inst
        .compile(&song)
        .map_err(|e| miette!("Compiler error: {}", e))?;

    let output_path = output.unwrap_or_else(|| {
        let mut path = input.clone();
        path.set_extension("mid");
        path
    });

    file::save_to_midi(&events, &output_path, song.metadata.bpm)?;
    println!("💾 Saved MIDI to {}", output_path.display());
    Ok(())
}
