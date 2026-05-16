use super::store::Store;
use crate::compiler::MidiEvent;
use crate::dsl::token::Frontmatter;
use crate::midi;
use midir::MidiOutputConnection;
use miette::{miette, Result};
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

struct ActiveNote {
    channel: u8,
    note: u8,
    off_time: f64,
}

pub struct Core {
    store: Store,
    conn: MidiOutputConnection,
    state: PlaybackState,
    start_time: Instant,
    seq_offset: f64, // Beat offset
    active_notes: Vec<ActiveNote>,
    last_processed_beat: f64,
    loop_range: Option<(f64, f64)>, // Start/End beats
}

impl Core {
    pub fn new(port_index: usize, client_name: &str) -> Result<Self> {
        let midi_out = midi::io::get_midi_output(client_name)?;
        let conn = midi::io::connect_out(midi_out, port_index, "loom-conn")?;

        Ok(Self {
            store: Store::new(),
            conn,
            state: PlaybackState::Stopped,
            start_time: Instant::now(),
            seq_offset: 0.0,
            active_notes: Vec::new(),
            last_processed_beat: -1.0,
            loop_range: None,
        })
    }

    pub fn load(&mut self, events: Vec<MidiEvent>, metadata: Frontmatter) {
        // If playing, adjust offset to prevent jumps
        if self.state == PlaybackState::Playing {
            let old_bpm = self.store.metadata.bpm.max(1) as f64;
            let elapsed = self.start_time.elapsed().as_secs_f64();
            self.seq_offset += elapsed * (old_bpm / 60.0);
            self.start_time = Instant::now();
        }
        self.loop_range = loop_range_beats(&metadata);
        self.store.update(events, metadata);
    }

    pub fn current_beat(&self) -> f64 {
        self.last_processed_beat.max(0.0)
    }

    pub fn set_loop_range(&mut self, start: f64, end: f64) {
        self.loop_range = Some((start, end));
    }

    pub fn play(&mut self) {
        if self.state != PlaybackState::Playing {
            let was_stopped = self.state == PlaybackState::Stopped;
            self.state = PlaybackState::Playing;
            self.start_time = Instant::now();
            if was_stopped {
                self.seq_offset = self.loop_range.map(|(s, _)| s).unwrap_or(0.0);
                self.last_processed_beat = self.seq_offset - 0.001; // Ensure start notes trigger
            }
        }
    }

    pub fn pause(&mut self) -> Result<()> {
        if self.state == PlaybackState::Playing {
            let bpm = self.store.metadata.bpm.max(1) as f64;
            self.seq_offset += self.start_time.elapsed().as_secs_f64() * (bpm / 60.0);
            self.state = PlaybackState::Paused;
            self.silence_all()?;
        }
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        self.state = PlaybackState::Stopped;
        self.seq_offset = 0.0;
        self.last_processed_beat = -1.0;
        self.silence_all()?;
        Ok(())
    }

    pub fn restart(&mut self) -> Result<()> {
        self.seq_offset = self.loop_range.map(|(s, _)| s).unwrap_or(0.0);
        self.last_processed_beat = self.seq_offset - 0.001;
        self.start_time = Instant::now();
        self.silence_all()?;
        Ok(())
    }

    pub fn preview_note(
        &mut self,
        channel: u8,
        note: u8,
        velocity: u8,
        duration: Duration,
    ) -> Result<bool> {
        if self.state == PlaybackState::Playing {
            return Ok(false);
        }

        self.send_midi(&[0x90 | channel, note, velocity])?;
        std::thread::sleep(duration);
        self.send_midi(&[0x80 | channel, note, 0])?;
        Ok(true)
    }

    pub fn tick(&mut self) -> Result<PlaybackState> {
        if self.state != PlaybackState::Playing {
            return Ok(self.state);
        }

        let bpm = self.store.metadata.bpm.max(1) as f64;
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let total_beats = (elapsed * (bpm / 60.0)) + self.seq_offset;

        // Determine Loop Boundaries
        let (loop_start, loop_end) = if let Some((s, e)) = self.loop_range {
            (s, e)
        } else {
            let max_time = self
                .store
                .events
                .iter()
                .filter_map(MidiEvent::note_end_time)
                .fold(0.0, f64::max)
                .max(4.0);
            (0.0, max_time)
        };

        let loop_len = loop_end - loop_start;
        // Current beat within the loop
        let current_beat = if self.store.metadata.r#loop {
            // Modulo arithmetic for looping
            loop_start + ((total_beats - loop_start) % loop_len)
        } else {
            if total_beats > loop_end {
                self.stop()?;
                return Ok(PlaybackState::Stopped);
            }
            total_beats
        };

        // Handle Wrap-around
        if current_beat < self.last_processed_beat {
            self.silence_all()?;
            self.last_processed_beat = loop_start - 0.001; // Reset
        }

        // Process Notes and Control events
        self.process_active_notes(current_beat)?;
        self.process_new_events(current_beat)?;

        self.last_processed_beat = current_beat;
        Ok(PlaybackState::Playing)
    }

    fn process_active_notes(&mut self, current_beat: f64) -> Result<()> {
        let mut i = 0;
        while i < self.active_notes.len() {
            if self.active_notes[i].off_time <= current_beat {
                let channel = self.active_notes[i].channel;
                let note = self.active_notes[i].note;
                self.send_midi(&[0x80 | channel, note, 0])?;
                self.active_notes.remove(i);
            } else {
                i += 1;
            }
        }
        Ok(())
    }

    fn process_new_events(&mut self, current_beat: f64) -> Result<()> {
        let events_to_emit: Vec<MidiEvent> = self
            .store
            .events
            .iter()
            .filter(|event| event.time() > self.last_processed_beat && event.time() <= current_beat)
            .cloned()
            .collect();

        for event in events_to_emit {
            match event {
                MidiEvent::Note {
                    time,
                    duration,
                    channel,
                    note,
                    velocity,
                } => {
                    self.send_midi(&[0x90 | channel, note, velocity])?;
                    self.active_notes.push(ActiveNote {
                        channel,
                        note,
                        off_time: time + duration,
                    });
                }
                MidiEvent::ControlChange {
                    channel, cc, value, ..
                } => {
                    self.send_midi(&[0xB0 | channel, cc, value])?;
                }
                MidiEvent::ProgramChange {
                    channel, program, ..
                } => {
                    self.send_midi(&[0xC0 | channel, program])?;
                }
            }
        }
        Ok(())
    }

    pub fn silence_all(&mut self) -> Result<()> {
        let mut first_error: Option<String> = None;

        for n in &self.active_notes {
            if let Err(e) = self.conn.send(&[0x80 | n.channel, n.note, 0]) {
                if first_error.is_none() {
                    first_error = Some(e.to_string());
                }
            }
        }
        self.active_notes.clear();

        // CC All Notes Off
        for i in 0..16 {
            if let Err(e) = self.conn.send(&[0xB0 | i, 123, 0]) {
                if first_error.is_none() {
                    first_error = Some(e.to_string());
                }
            }
        }

        if let Some(msg) = first_error {
            return Err(miette!("MIDI send failed while silencing: {}", msg));
        }

        Ok(())
    }

    fn send_midi(&mut self, data: &[u8]) -> Result<()> {
        self.conn
            .send(data)
            .map_err(|e| miette!("MIDI send failed: {}", e))
    }
}

fn loop_range_beats(metadata: &Frontmatter) -> Option<(f64, f64)> {
    let range = metadata.loop_range.as_ref()?;
    let (start, end) = crate::validation::parse_loop_range_units(range).ok()?;
    let beats_per_unit =
        crate::validation::beats_per_unit(&metadata.unit, &metadata.signature).ok()?;
    Some((start * beats_per_unit, end * beats_per_unit))
}

#[cfg(test)]
mod tests {
    use super::loop_range_beats;
    use crate::dsl::token::Frontmatter;

    #[test]
    fn loop_range_beats_converts_bar_units() {
        let metadata = Frontmatter {
            signature: "3/4".to_string(),
            unit: "bar".to_string(),
            loop_range: Some("1..3".to_string()),
            ..Frontmatter::default()
        };

        assert_eq!(loop_range_beats(&metadata), Some((3.0, 9.0)));
    }

    #[test]
    fn loop_range_beats_keeps_beat_units() {
        let metadata = Frontmatter {
            signature: "3/4".to_string(),
            unit: "beat".to_string(),
            loop_range: Some("3..9".to_string()),
            ..Frontmatter::default()
        };

        assert_eq!(loop_range_beats(&metadata), Some((3.0, 9.0)));
    }

    #[test]
    fn loop_range_beats_is_none_without_range() {
        assert_eq!(loop_range_beats(&Frontmatter::default()), None);
    }
}
