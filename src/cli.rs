use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Check syntax of Loom file (CI/CD, Validation)
    Check { input: PathBuf },
    /// Parse and output MIDI events (Dry run, formerly Run)
    Parse {
        input: PathBuf,
        /// Output format
        #[arg(long, value_enum, default_value_t = ParseFormat::Table)]
        format: ParseFormat,
        /// Sort key
        #[arg(long, value_enum, default_value_t = ParseSortKey::Time)]
        sort: ParseSortKey,
        /// Filter(s), e.g. --filter channel=2,note=77 --filter velocity_min=60
        #[arg(long, value_name = "KEY=VALUE[,KEY=VALUE...]")]
        filter: Vec<String>,
        /// Print summary
        #[arg(long)]
        summary: bool,
    },
    /// Real-time MIDI Playback (One-shot)
    Play {
        input: PathBuf,
        /// MIDI output port index
        #[arg(short, long, default_value_t = 0)]
        port: usize,
    },
    /// Interactive Live Coding Mode (TUI & Hot-swap)
    Live {
        input: PathBuf,
        /// MIDI output port index
        #[arg(short, long, default_value_t = 0)]
        port: usize,
    },
    /// Export to MIDI file
    Save {
        input: PathBuf,
        /// Output file path. If not provided, defaults to input filename with .mid extension
        output: Option<PathBuf>,
    },
    /// Format Loom file
    Fmt {
        /// Input file. If not provided, reads from stdin.
        input: Option<PathBuf>,
        /// Default formatter check mode
        #[arg(long, short)]
        check: bool,
    },
    /// List available MIDI output ports
    Ports,
}

#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum ParseFormat {
    Table,
    Json,
    Csv,
}

#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum ParseSortKey {
    Time,
    Note,
    Channel,
    Velocity,
    Duration,
    Track,
}
