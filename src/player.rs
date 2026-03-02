use crate::compiler::MidiEvent;
use crate::sequencer::{Core, PlaybackState};
use miette::Result;
use std::{thread, time::Duration};

pub struct Player {
    // Keep connection here? Or let Core handle it?
    // Core handles it now. But Player::new was returning Self { conn }.
    // We need to change Player to holding Core or just a wrapper.
    core: Core,
}

impl Player {
    pub fn new(port_index: usize) -> Result<Self> {
        let core = Core::new(port_index, "Loom Output")?;
        Ok(Self { core })
    }

    pub fn play(
        &mut self,
        events: &[MidiEvent],
        metadata: &crate::dsl::token::Frontmatter,
    ) -> Result<()> {
        println!("Playing at {} BPM...", metadata.bpm);

        // Load data
        self.core.load(events.to_vec(), metadata.clone());

        // Setup loop range if provided
        if let Some(ref range_str) = metadata.loop_range {
            if let Ok((start, end)) =
                parse_loop_range(range_str, &metadata.unit, &metadata.signature)
            {
                self.core.set_loop_range(start, end);
                println!("Loop Range: {} ~ {} beats", start, end);
            }
        }

        self.core.play();

        let tick_rate = Duration::from_millis(5);

        loop {
            let state = self.core.tick();
            if state == PlaybackState::Stopped {
                break;
            }
            thread::sleep(tick_rate);
        }

        println!("Done.");
        Ok(())
    }
}

// Helper to parse "1 ~ 4" from metadata
fn parse_loop_range(range_str: &str, default_unit: &str, signature: &str) -> Result<(f64, f64)> {
    use miette::{miette, IntoDiagnostic};

    // Split by '~'
    let parts: Vec<&str> = range_str.split('~').collect();

    if parts.len() != 2 {
        return Err(miette!(
            "Invalid loop_range format. Expected 'start ~ end' (e.g. '1 ~ 4'), got '{}'",
            range_str
        ));
    }

    let start_val = parts[0].trim().parse::<f64>().into_diagnostic()?;
    let end_val = parts[1].trim().parse::<f64>().into_diagnostic()?;

    let beats_per_unit = get_beats_per_unit(default_unit, signature);

    // Convert 1-based unit index to 0-based beats
    // Start is inclusive (beginning of unit), End is inclusive (end of unit)
    let start_beats = (start_val - 1.0).max(0.0) * beats_per_unit;
    let end_beats = end_val * beats_per_unit;

    Ok((start_beats, end_beats))
}

fn get_beats_per_unit(unit: &str, signature: &str) -> f64 {
    match unit.to_lowercase().as_str() {
        "bar" => {
            // parse signature "4/4" -> 4 beats
            let top: f64 = signature
                .split('/')
                .next()
                .unwrap_or("4")
                .parse()
                .unwrap_or(4.0);
            top
        }
        "beat" => 1.0,
        _ => 4.0, // default to bar
    }
}
