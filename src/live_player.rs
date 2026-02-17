use crate::compiler::MidiEvent;
use crate::token::Frontmatter;
use midir::MidiOutput;
use miette::{miette, IntoDiagnostic, Result};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

pub enum PlayerCommand {
    UpdateSequence(Vec<MidiEvent>, Frontmatter),
    Play,
    Pause,
    Stop,
}

pub struct LivePlayer {
    command_sender: Sender<PlayerCommand>,
}

impl LivePlayer {
    pub fn new(port_index: usize) -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            if let Err(e) = run_player_loop(port_index, rx) {
                eprintln!("Player thread error: {}", e);
            }
        });

        Ok(Self { command_sender: tx })
    }

    pub fn update(&self, events: Vec<MidiEvent>, metadata: Frontmatter) {
        let _ = self
            .command_sender
            .send(PlayerCommand::UpdateSequence(events, metadata));
    }

    pub fn play(&self) {
        let _ = self.command_sender.send(PlayerCommand::Play);
    }

    pub fn pause(&self) {
        let _ = self.command_sender.send(PlayerCommand::Pause);
    }

    pub fn stop(&self) {
        let _ = self.command_sender.send(PlayerCommand::Stop);
    }
}

struct PlayerState {
    events: Vec<MidiEvent>,
    metadata: Frontmatter,
    is_playing: bool,
    start_time: Instant,
    seq_offset: f64, // Offset in beats when started/resumed
}

struct ActiveNote {
    channel: u8,
    note: u8,
    off_time: f64, // Absolute beat time
}

fn run_player_loop(port_index: usize, rx: Receiver<PlayerCommand>) -> Result<()> {
    let midi_out = MidiOutput::new("Loom Live").into_diagnostic()?;
    let ports = midi_out.ports();
    let port = ports
        .get(port_index)
        .ok_or_else(|| miette!("Invalid port index"))?;
    let mut conn = midi_out
        .connect(port, "loom-live")
        .map_err(|e| miette!("Connection error: {}", e))?;

    let mut state = PlayerState {
        events: Vec::new(),
        metadata: Frontmatter::default(),
        is_playing: false,
        start_time: Instant::now(),
        seq_offset: 0.0,
    };

    let tick_rate = Duration::from_millis(5); // 5ms precision
    let mut last_processed_beat = -1.0;
    let mut active_notes: Vec<ActiveNote> = Vec::new();

    loop {
        // Handle Commands
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                PlayerCommand::UpdateSequence(events, metadata) => {
                    // To avoid timing jump, bake current elapsed time into offset at OLD bpm
                    if state.is_playing {
                        let old_bpm = state.metadata.bpm.max(1) as f64;
                        let old_beats_per_sec = old_bpm / 60.0;
                        state.seq_offset +=
                            state.start_time.elapsed().as_secs_f64() * old_beats_per_sec;
                        state.start_time = Instant::now();
                    }

                    state.events = events;
                    state.metadata = metadata;
                    // Reset processed beat logic if needed?
                    // Probably reset last_processed_beat to current loop phase to avoid double triggering?
                    // But if we just update events, we continue from current beat.
                }
                PlayerCommand::Play => {
                    if !state.is_playing {
                        state.is_playing = true;
                        state.start_time = Instant::now();
                    }
                }
                PlayerCommand::Pause => {
                    if state.is_playing {
                        state.is_playing = false;
                        // Calculate offset to resume later
                        let bpm = state.metadata.bpm.max(1) as f64;
                        let beats_per_sec = bpm / 60.0;
                        state.seq_offset +=
                            state.start_time.elapsed().as_secs_f64() * beats_per_sec;

                        silence_all(&mut conn, &mut active_notes);
                    }
                }
                PlayerCommand::Stop => {
                    silence_all(&mut conn, &mut active_notes);
                    return Ok(());
                }
            }
        }

        if state.is_playing {
            // Calculate current beat
            let elapsed = state.start_time.elapsed();
            let bpm = state.metadata.bpm.max(1) as f64;
            let beats_per_sec = bpm / 60.0;
            let total_beats = (elapsed.as_secs_f64() * beats_per_sec) + state.seq_offset;

            let max_beat = state
                .events
                .iter()
                .map(|e| e.time + e.duration)
                .fold(0.0, f64::max)
                .max(4.0);

            // Handle Loop
            let current_beat = if state.metadata.r#loop {
                total_beats % max_beat
            } else {
                if total_beats > max_beat {
                    state.is_playing = false;
                    // All notes off
                    active_notes.clear();
                    for i in 0..16 {
                        let _ = conn.send(&[0xB0 | i, 123, 0]);
                    }
                    continue;
                }
                total_beats
            };

            // Detect Loop Wrap
            if current_beat < last_processed_beat {
                last_processed_beat = -1.0;
                silence_all(&mut conn, &mut active_notes);
            }

            // Note Off from Active Notes
            // We check if off_time <= current_beat
            active_notes.retain(|n| {
                if n.off_time <= current_beat {
                    let _ = conn.send(&[0x80 | n.channel, n.note, 0]); // Note Off
                    false // Remove
                } else {
                    true // Keep
                }
            });

            // Note On
            for event in &state.events {
                // Determine if event started in (last, current]
                // Also handle case where event started exactly at 0.0 (if last is -1.0)
                if event.time > last_processed_beat && event.time <= current_beat {
                    let channel = event.channel.min(15);
                    let note = event.note;
                    let _ = conn.send(&[0x90 | channel, note, 100]); // Note On

                    active_notes.push(ActiveNote {
                        channel,
                        note,
                        off_time: event.time + event.duration,
                    });
                }
            }

            last_processed_beat = current_beat;
        }

        thread::sleep(tick_rate);
    }
}

fn silence_all(conn: &mut midir::MidiOutputConnection, active_notes: &mut Vec<ActiveNote>) {
    // 1. Explicit Note Off for tracked active notes
    for note in active_notes.iter() {
        let _ = conn.send(&[0x80 | note.channel.min(15), note.note, 0]);
    }
    active_notes.clear();

    // 2. All Channels Silence Commands
    for i in 0..16 {
        let channel = i as u8;
        // CC 120: All Sound Off (Immediate silence)
        let _ = conn.send(&[0xB0 | channel, 120, 0]);
        // CC 123: All Notes Off (Release phase)
        let _ = conn.send(&[0xB0 | channel, 123, 0]);
        // CC 64: Sustain Pedal Off (0)
        let _ = conn.send(&[0xB0 | channel, 64, 0]);
        // CC 121: Reset All Controllers
        let _ = conn.send(&[0xB0 | channel, 121, 0]);
    }
}
