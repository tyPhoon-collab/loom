use midly::{Format, Header, MidiMessage, Smf, Timing, TrackEvent};
use miette::{IntoDiagnostic, Result};
use std::path::Path;

use crate::compiler::MidiEvent;

pub fn save_to_midi(events: &[MidiEvent], path: &Path, bpm: u32) -> Result<()> {
    // 1. Create SMF Header
    // generic typical TPQN (Ticks Per Quarter Note)
    let ppqn = 480;
    let header = Header::new(Format::SingleTrack, Timing::Metrical(ppqn.into()));

    // 2. Convert Loom MidiEvents to Midly TrackEvents
    // Loom events are absolute time in seconds (f64).
    // MIDI events are delta time in ticks.

    // Calculate ticks per second based on BPM and PPQN
    // BPM = Beats Per Minute
    // PPQN = Ticks Per Beat
    // Ticks Per Minute = BPM * PPQN
    // Ticks Per Second = (BPM * PPQN) / 60

    let ticks_per_second = (bpm as f64 * ppqn as f64) / 60.0;

    // Sort events by time just in case
    let mut sorted_events = events.to_vec();
    sorted_events.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());

    // We need to handle NoteOn and NoteOff.
    // Loom's MidiEvent has duration, so it implies NoteOn at `time` and NoteOff at `time + duration`.

    #[derive(Debug)]
    struct TempEvent {
        time: f64,
        kind: TempEventKind,
        channel: u8,
        note: u8,
    }

    #[derive(Debug)]
    enum TempEventKind {
        NoteOn,
        NoteOff,
    }

    let mut temp_events = Vec::new();
    for event in &sorted_events {
        temp_events.push(TempEvent {
            time: event.time,
            kind: TempEventKind::NoteOn,
            channel: event.channel,
            note: event.note,
        });
        temp_events.push(TempEvent {
            time: event.time + event.duration,
            kind: TempEventKind::NoteOff,
            channel: event.channel,
            note: event.note,
        });
    }

    // Sort temp events by time
    temp_events.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());

    let mut track = Vec::new();
    let mut current_time = 0.0;
    // let mut current_ticks = 0;

    // Add Tempo Meta Event (Optional but good practice)
    // For now, let's just emit notes.

    for event in temp_events {
        let delta_time = event.time - current_time;
        // Ensure delta_time is non-negative (floating point precision issues?)
        let delta_time = delta_time.max(0.0);
        let delta_ticks = (delta_time * ticks_per_second).round() as u32;

        let kind = match event.kind {
            TempEventKind::NoteOn => MidiMessage::NoteOn {
                key: event.note.into(),
                vel: 64.into(), // Default velocity
            },
            TempEventKind::NoteOff => MidiMessage::NoteOff {
                key: event.note.into(),
                vel: 0.into(),
            },
        };

        track.push(TrackEvent {
            delta: delta_ticks.into(),
            kind: midly::TrackEventKind::Midi {
                channel: event.channel.into(),
                message: kind,
            },
        });

        current_time = event.time;
        // current_ticks += delta_ticks;
    }

    // Add End of Track
    track.push(TrackEvent {
        delta: 0.into(),
        kind: midly::TrackEventKind::Meta(midly::MetaMessage::EndOfTrack),
    });

    let smf = Smf {
        header,
        tracks: vec![track],
    };

    // 3. Write to file
    smf.save(path).into_diagnostic()?;

    Ok(())
}
