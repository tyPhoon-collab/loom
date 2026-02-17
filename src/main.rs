use clap::{Parser, Subcommand};
use loom::{compiler, parser, player};
use miette::{miette, IntoDiagnostic, Result};
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
    /// Format Loom file
    Fmt {
        /// Input file. If not provided, reads from stdin.
        input: Option<PathBuf>,
        /// Check mode: exits with 1 if formatting changes content
        #[arg(long, short)]
        check: bool,
    },
}

#[derive(Tabled)]
struct MidiEventRow {
    #[tabled(rename = "CH")]
    channel: u8,
    #[tabled(rename = "Note")]
    note: u8,
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
            let compiler_inst = compiler::Compiler::new(&song);
            let events = compiler_inst
                .compile(&song)
                .map_err(|e| miette!("Compiler error: {}", e))?;

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
            let compiler_inst = compiler::Compiler::new(&song);
            let events = compiler_inst
                .compile(&song)
                .map_err(|e| miette!("Compiler error: {}", e))?;

            let mut player_inst = player::Player::new(port)?;
            player_inst.play(&events, &song.metadata)?;
        }
        Commands::Fmt { input, check } => {
            use loom::formatter;
            use std::io::{self, Read};

            let (content, path_str) = match &input {
                Some(path) => (
                    fs::read_to_string(path).into_diagnostic()?,
                    path.display().to_string(),
                ),
                None => {
                    let mut buffer = String::new();
                    io::stdin().read_to_string(&mut buffer).into_diagnostic()?;
                    (buffer, "<stdin>".to_string())
                }
            };

            let formatted = formatter::format_string(&content);

            if check {
                if content != formatted {
                    eprintln!("Difference found in {}", path_str);
                    std::process::exit(1);
                }
            } else {
                // If input was file, write back to file?
                // Standard behavior for `fmt <file>` is usually overwrite?
                // `cargo fmt` overwrites. `prettier <file>` usually writes to stdout unless --write.
                // Let's follow `cargo fmt` / `go fmt` style if file is provided: OVERWRITE.
                // If stdin, write to stdout.

                match input {
                    Some(path) => {
                        fs::write(&path, formatted).into_diagnostic()?;
                        eprintln!("Formatted {}", path.display());
                    }
                    None => {
                        print!("{}", formatted);
                    }
                }
            }
        }
    }

    Ok(())
}
