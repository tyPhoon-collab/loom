use loom::compiler;
use loom::dsl::parser;
use loom::midi::file;
use miette::{miette, IntoDiagnostic, Result};
use std::fs;
use std::path::PathBuf;

pub fn handle_save(input: PathBuf, output: Option<PathBuf>) -> Result<()> {
    let content = fs::read_to_string(&input).into_diagnostic()?;
    let song = parser::parse_song(content)?;
    let compiler_inst = compiler::Compiler::new(&song)?;
    let note_events = compiler_inst
        .compile(&song)
        .map_err(|e| miette!("Compiler error: {}", e))?;
    let init_events = compiler::collect_init_events(&song);

    let output_path = output.unwrap_or_else(|| {
        let mut path = input.clone();
        path.set_extension("mid");
        path
    });

    file::save_to_midi(&note_events, &init_events, &output_path, song.metadata.bpm)?;
    println!("💾 Saved MIDI to {}", output_path.display());
    Ok(())
}
