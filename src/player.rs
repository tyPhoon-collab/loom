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

        if let Some(ref range_str) = metadata.loop_range {
            println!("Loop Range: {}", range_str);
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
