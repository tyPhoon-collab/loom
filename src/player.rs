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
        use miette::miette;

        println!("Playing at {} BPM...", metadata.bpm);

        // Load data
        self.core.load(events.to_vec(), metadata.clone());

        // Setup loop range if provided
        if let Some(ref range_str) = metadata.loop_range {
            let (start, end) = parse_loop_range(range_str, &metadata.unit, &metadata.signature)
                .map_err(|e| miette!("Invalid loop_range: {}", e))?;
            self.core.set_loop_range(start, end);
            println!("Loop Range: {} ~ {} beats", start, end);
        }

        self.core.play();

        let tick_rate = Duration::from_millis(5);

        loop {
            let state = self.core.tick()?;
            if state == PlaybackState::Stopped {
                break;
            }
            thread::sleep(tick_rate);
        }

        println!("Done.");
        Ok(())
    }
}

// Helper to parse "0..4" from metadata
fn parse_loop_range(range_str: &str, default_unit: &str, signature: &str) -> Result<(f64, f64)> {
    use miette::miette;

    let (start_val, end_val) =
        crate::validation::parse_loop_range_units(range_str).map_err(|e| miette!("{}", e))?;
    let beats_per_unit =
        crate::validation::beats_per_unit(default_unit, signature).map_err(|e| miette!("{}", e))?;

    // Convert half-open unit range to beats: start inclusive, end exclusive.
    let start_beats = start_val * beats_per_unit;
    let end_beats = end_val * beats_per_unit;

    Ok((start_beats, end_beats))
}
