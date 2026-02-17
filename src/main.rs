mod compiler;
mod parser;
mod token;
mod player; // Add module

use clap::{Parser, Subcommand};
use miette::{IntoDiagnostic, Result, miette};
use std::fs;
use std::path::PathBuf;
use tabled::{Table, Tabled};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Check syntax of Loom file
    Check { input: PathBuf },
    /// Run Loom file and output MIDI events (Dry run)
    Run { input: PathBuf },
    /// Real-time MIDI Playback
    Play {
        input: PathBuf,
        /// MIDI output port index
        #[arg(short, long, default_value_t = 0)]
        port: usize,
    },
}

#[derive(Tabled)]
struct MidiEventRow {
    #[tabled(rename = "CH")]
    channel: u8,
    #[tabled(rename = "Note")]
    note: String,
    // velocity: u8,
    #[tabled(rename = "Time")]
    time: String,
    #[tabled(rename = "Duration")]
    duration: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { input } => {
            let content = fs::read_to_string(&input).into_diagnostic()?;
            match parser::parse_song(content) {
                Ok(song) => {
                    println!("✅ Syntax OK: {}", input.display());
                    println!("   Title: {:?}", song.metadata.title);
                    println!("   BPM: {}", song.metadata.bpm);
                    println!("   Tracks: {}", song.tracks.len());
                }
                Err(e) => return Err(e.into()),
            }
        }
        Commands::Run { input } => {
            let content = fs::read_to_string(&input).into_diagnostic()?;
            let song = parser::parse_song(content)?;
            let compiler = compiler::Compiler::new(&song);
            let events = compiler.compile(&song).map_err(|e| miette!("Compiler error: {}", e))?;

            // Output Table
            let mut rows = Vec::new();
            for event in events {
                rows.push(MidiEventRow {
                    channel: event.channel,
                    note: event.note,
                    time: format!("{:.2}", event.time),
                    duration: format!("{:.2}", event.duration),
                });
            }
            let table = Table::new(rows).to_string();
            println!("{}", table);
        }
        Commands::Play { input, port } => {
            let content = fs::read_to_string(&input).into_diagnostic()?;
            let song = parser::parse_song(content)?;
            let compiler = compiler::Compiler::new(&song);
            let events = compiler.compile(&song).map_err(|e| miette!("Compiler error: {}", e))?;

            let mut player = player::Player::new(port)?;
            player.play(&events, song.metadata.bpm)?;
        }
    }

    Ok(())
}
