use super::store::Store;
use crate::compiler::MidiEvent;
use crate::dsl::token::Frontmatter;
use crate::midi;
use midir::MidiOutputConnection;
use miette::Result;
use std::time::Instant;

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
            self.state = PlaybackState::Playing;
            self.start_time = Instant::now();
            if self.state == PlaybackState::Stopped {
                self.seq_offset = self.loop_range.map(|(s, _)| s).unwrap_or(0.0);
                self.last_processed_beat = self.seq_offset - 0.001; // Ensure start notes trigger
            }
        }
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            let bpm = self.store.metadata.bpm.max(1) as f64;
            self.seq_offset += self.start_time.elapsed().as_secs_f64() * (bpm / 60.0);
            self.state = PlaybackState::Paused;
            self.silence_all();
        }
    }

    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.seq_offset = 0.0;
        self.last_processed_beat = -1.0;
        self.silence_all();
    }

    pub fn restart(&mut self) {
        self.seq_offset = self.loop_range.map(|(s, _)| s).unwrap_or(0.0);
        self.last_processed_beat = self.seq_offset - 0.001;
        self.start_time = Instant::now();
        self.silence_all();
    }

    pub fn tick(&mut self) -> PlaybackState {
        if self.state != PlaybackState::Playing {
            return self.state;
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
                self.stop();
                return PlaybackState::Stopped;
            }
            total_beats
        };

        // Handle Wrap-around
        if current_beat < self.last_processed_beat {
            self.silence_all();
            self.last_processed_beat = loop_start - 0.001; // Reset
        }

        // Process Notes and Control events
        self.process_active_notes(current_beat);
        self.process_new_events(current_beat);

        self.last_processed_beat = current_beat;
        PlaybackState::Playing
    }

    fn process_active_notes(&mut self, current_beat: f64) {
        let conn = &mut self.conn;
        self.active_notes.retain(|n| {
            if n.off_time <= current_beat {
                let _ = conn.send(&[0x80 | n.channel, n.note, 0]);
                false
            } else {
                true
            }
        });
    }

    fn process_new_events(&mut self, current_beat: f64) {
        for event in &self.store.events {
            if event.time() > self.last_processed_beat && event.time() <= current_beat {
                match event {
                    MidiEvent::Note {
                        time,
                        duration,
                        channel,
                        note,
                        velocity,
                    } => {
                        let channel = (*channel).min(15);
                        let _ = self.conn.send(&[0x90 | channel, *note, *velocity]);
                        self.active_notes.push(ActiveNote {
                            channel,
                            note: *note,
                            off_time: *time + *duration,
                        });
                    }
                    MidiEvent::ControlChange {
                        channel, cc, value, ..
                    } => {
                        let ch = (*channel).min(15);
                        let _ = self.conn.send(&[0xB0 | ch, *cc, *value]);
                    }
                    MidiEvent::ProgramChange {
                        channel, program, ..
                    } => {
                        let ch = (*channel).min(15);
                        let _ = self.conn.send(&[0xC0 | ch, *program]);
                    }
                }
            }
        }
    }

    pub fn silence_all(&mut self) {
        for n in &self.active_notes {
            let _ = self.conn.send(&[0x80 | n.channel, n.note, 0]);
        }
        self.active_notes.clear();

        // CC All Notes Off
        for i in 0..16 {
            let _ = self.conn.send(&[0xB0 | i, 123, 0]);
        }
    }
}
