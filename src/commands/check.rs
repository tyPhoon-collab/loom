use loom::dsl::parser;
use miette::{IntoDiagnostic, Result};
use std::fs;
use std::path::PathBuf;

pub fn handle_check(input: PathBuf) -> Result<()> {
    let content = fs::read_to_string(&input).into_diagnostic()?;
    match parser::parse_song(content) {
        Ok(song) => {
            println!("✅ Syntax OK: {}", input.display());
            println!("   Title: {:?}", song.metadata.title);
            println!("   BPM: {}", song.metadata.bpm);
            println!("   Tracks: {}", song.tracks.len());
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
