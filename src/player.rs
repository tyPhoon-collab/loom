use crate::compiler::MidiEvent;
use midir::{MidiOutput, MidiOutputConnection};
use miette::{miette, IntoDiagnostic, Result};
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
        for (i, port) in ports.iter().enumerate() {
            println!("  {}: {}", i, midi_out.port_name(port).unwrap_or_default());
        }

        let port = ports.get(port_index).ok_or_else(|| {
            miette!(
                "Port index {} is out of range. Please choose from the list above.",
                port_index
            )
        })?;

        println!(
            "Connecting to port {}: {}",
            port_index,
            midi_out.port_name(port).unwrap_or_default()
        );

        let conn = midi_out
            .connect(port, "loom-conn")
            .map_err(|e| miette!("Connection error: {}", e))?;

        Ok(Self { conn })
    }

    pub fn play(
        &mut self,
        compiled_events: &[MidiEvent],
        metadata: &crate::token::Frontmatter,
    ) -> Result<()> {
        let bpm = metadata.bpm;
        let loop_flag = metadata.r#loop;

        let (start_beat, end_beat) = if let Some(ref range_str) = metadata.loop_range {
            parse_loop_range(range_str, &metadata.unit, &metadata.signature)?
        } else {
            // Default: 0 to max time
            let max_time = compiled_events
                .iter()
                .map(|e| e.time + e.duration)
                .fold(0.0, f64::max);
            (0.0, max_time)
        };

        let loop_duration_beats = end_beat - start_beat;

        // Filter and offset events
        let mut filtered_events = Vec::new();
        for e in compiled_events {
            if e.time >= start_beat && e.time < end_beat {
                let mut new_e = e.clone();
                new_e.time -= start_beat;
                // Clamp duration to end of range
                if new_e.time + new_e.duration > loop_duration_beats {
                    new_e.duration = loop_duration_beats - new_e.time;
                }
                filtered_events.push(new_e);
            }
        }

        println!(
            "Playing at {} BPM... ({} beats segment)",
            bpm, loop_duration_beats
        );
        if metadata.loop_range.is_some() {
            println!("Range: {} beats to {} beats", start_beat, end_beat);
        }

        loop {
            // Flatten into (NoteOn / NoteOff) events
            let mut play_events = Vec::new();
            for e in &filtered_events {
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
                    time: e.time + e.duration,
                    kind: PlayEventKind::NoteOff,
                    channel,
                    note: note_num,
                    velocity: 0,
                });
            }

            // Sort by time
            play_events.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(Ordering::Equal));

            // Play segment
            let mut current_beat = 0.0;
            let ms_per_beat = 60000.0 / bpm as f64;

            for event in play_events {
                let delta = event.time - current_beat;
                if delta > 0.001 {
                    let sleep_ms = delta * ms_per_beat;
                    thread::sleep(Duration::from_millis(sleep_ms as u64));
                    current_beat = event.time;
                }

                let status = match event.kind {
                    PlayEventKind::NoteOn => 0x90 | event.channel,
                    PlayEventKind::NoteOff => 0x80 | event.channel,
                };
                let _ = self.conn.send(&[status, event.note, event.velocity]);
            }

            // Sleep until the end of the segment duration
            let final_delta = loop_duration_beats - current_beat;
            if final_delta > 0.001 {
                thread::sleep(Duration::from_millis((final_delta * ms_per_beat) as u64));
            }

            if !loop_flag {
                break;
            }
            println!("Looping...");
            // Send All Notes Off roughly?
            // Or just rely on the NoteOffs we sent.
            // Since we clamped durations, all NoteOffs should have been sent before loop_duration_beats.
        }

        println!("Done.");
        Ok(())
    }
}

// Helper to parse "1 ~ 4"
fn parse_loop_range(range_str: &str, default_unit: &str, signature: &str) -> Result<(f64, f64)> {
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

fn convert_note_to_midi(note_name: &str) -> u8 {
    match note_name.to_lowercase().as_str() {
        "c3" => 60,
        "c#3" => 61,
        "d3" => 62,
        "d#3" => 63,
        "e3" => 64,
        "f3" => 65,
        "f#3" => 66,
        "g3" => 67,
        "g#3" => 68,
        "a3" => 69,
        "a#3" => 70,
        "b3" => 71,
        "c4" => 72,
        "c2" => 48,
        "d2" => 50,
        "e2" => 52,
        "f2" => 53,
        "g2" => 55,
        "a2" => 57,
        "b2" => 59,
        "kick" => 36,
        "snare" => 38,
        "hi-hat" | "hihat" => 42,
        _ => 60,
    }
}
