use crate::compiler::MidiEvent;
use midir::{MidiOutput, MidiOutputConnection};
use miette::{IntoDiagnostic, Result, miette};
use std::{cmp::Ordering, thread, time::Duration};

pub struct Player {
    conn: MidiOutputConnection,
}

#[derive(Debug)]
struct PlayEvent {
    time: f64,
    kind: PlayEventKind,
    channel: u8,
    note: u8,
    velocity: u8,
}

#[derive(Debug)]
enum PlayEventKind {
    NoteOn,
    NoteOff,
}

impl Player {
    pub fn new(port_index: usize) -> Result<Self> {
        let midi_out = MidiOutput::new("Loom Output").into_diagnostic()?;

        let ports = midi_out.ports();

        println!("Available ports:");
        for i in 0..ports.len() {
             println!("  {}: {}", i, midi_out.port_name(&ports[i]).unwrap_or_default());
        }

        let port = ports.get(port_index).ok_or_else(|| {
            miette!("Port index {} is out of range. Please choose from the list above.", port_index)
        })?;

        println!("Connecting to port {}: {}", port_index, midi_out.port_name(port).unwrap_or_default());

        let conn = midi_out.connect(port, "loom-conn").map_err(|e| miette!("Connection error: {}", e))?;

        Ok(Self { conn })
    }

    pub fn play(&mut self, compiled_events: &[MidiEvent], bpm: u32) -> Result<()> {
        println!("Playing at {} BPM...", bpm);

        // 1. Flatten into (NoteOn / NoteOff) events
        let mut play_events = Vec::new();
        for e in compiled_events {
            let note_num = convert_note_to_midi(&e.note);
            let channel = (e.channel - 1).min(15);

            play_events.push(PlayEvent {
                time: e.time,
                kind: PlayEventKind::NoteOn,
                channel,
                note: note_num,
                velocity: 100,
            });

            play_events.push(PlayEvent {
                time: e.time + e.duration, // Note Off time
                kind: PlayEventKind::NoteOff,
                channel,
                note: note_num,
                velocity: 0,
            });
        }

        // 2. Sort by time
        play_events.sort_by(|a, b| {
            a.time.partial_cmp(&b.time).unwrap_or(Ordering::Equal)
        });

        // 3. Play loop
        let mut current_time = 0.0;
        let ms_per_beat = 60000.0 / bpm as f64;

        for event in play_events {
            // Wait for delta
            let delta = event.time - current_time;
            if delta > 0.001 {
                let sleep_ms = delta * ms_per_beat;
                thread::sleep(Duration::from_millis(sleep_ms as u64));
                current_time = event.time;
            }

            // Send message
            let status = match event.kind {
                PlayEventKind::NoteOn => 0x90 | event.channel,
                PlayEventKind::NoteOff => 0x80 | event.channel,
            };

            let _ = self.conn.send(&[status, event.note, event.velocity]);
        }

        println!("Done.");
        Ok(())
    }
}

fn convert_note_to_midi(note_name: &str) -> u8 {
    match note_name.to_lowercase().as_str() {
        "c3" => 60, "c#3" => 61, "d3" => 62, "d#3" => 63, "e3" => 64, "f3" => 65, "f#3" => 66, "g3" => 67, "g#3" => 68, "a3" => 69, "a#3" => 70, "b3" => 71,
        "c4" => 72,
        "c2" => 48, "d2" => 50, "e2" => 52, "f2" => 53, "g2" => 55, "a2" => 57, "b2" => 59,
        "kick" => 36, "snare" => 38, "hi-hat" | "hihat" => 42,
        _ => 60,
    }
}
