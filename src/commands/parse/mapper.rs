use super::model::{midi_note_name, ParsedEvent};
use loom::compiler::MidiEvent;
use loom::inspect::TrackEvent;

pub(super) fn to_parsed_events(track_events: Vec<TrackEvent>) -> Vec<ParsedEvent> {
    track_events
        .into_iter()
        .map(|te| match te.event {
            MidiEvent::Note {
                time,
                duration,
                channel,
                note,
                velocity,
            } => ParsedEvent {
                event_type: "note".to_string(),
                track: te.track,
                channel: channel.saturating_add(1),
                note: Some(note),
                note_name: Some(midi_note_name(note)),
                velocity: Some(velocity),
                cc: None,
                value: None,
                program: None,
                time,
                duration: Some(duration),
                end_time: Some(time + duration),
            },
            MidiEvent::ControlChange {
                time,
                channel,
                cc,
                value,
            } => ParsedEvent {
                event_type: "cc".to_string(),
                track: te.track,
                channel: channel.saturating_add(1),
                note: None,
                note_name: None,
                velocity: None,
                cc: Some(cc),
                value: Some(value),
                program: None,
                time,
                duration: None,
                end_time: None,
            },
            MidiEvent::ProgramChange {
                time,
                channel,
                program,
            } => ParsedEvent {
                event_type: "pc".to_string(),
                track: te.track,
                channel: channel.saturating_add(1),
                note: None,
                note_name: None,
                velocity: None,
                cc: None,
                value: None,
                program: Some(program),
                time,
                duration: None,
                end_time: None,
            },
        })
        .collect()
}
