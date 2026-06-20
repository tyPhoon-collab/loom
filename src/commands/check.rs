use loom::dsl::parser;
use miette::Result;
use std::path::PathBuf;

pub fn handle_check(input: PathBuf) -> Result<()> {
    match parser::parse_song_from_path(&input) {
        Ok(song) => {
            println!("✅ Syntax OK: {}", input.display());
            println!("   Title: {:?}", song.metadata.title);
            println!("   BPM: {}", song.metadata.bpm);
            println!("   Tracks: {}", song.tracks.len());
            Ok(())
        }
        Err(e) => Err(e),
    }
}
