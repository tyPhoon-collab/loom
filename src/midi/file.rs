use midly::{Format, Header, MidiMessage, Smf, Timing, TrackEvent};
use miette::{IntoDiagnostic, Result};
use std::path::Path;

use crate::compiler::MidiEvent;

pub fn save_to_midi(events: &[MidiEvent], path: &Path, bpm: u32) -> Result<()> {
    // 1. Create SMF Header
    // generic typical TPQN (Ticks Per Quarter Note)
    let ppqn = 480;
    let header = Header::new(Format::SingleTrack, Timing::Metrical(ppqn.into()));

    // 2. Convert Loom MidiEvents to Midly TrackEvents.
    let ticks_per_second = (bpm as f64 * ppqn as f64) / 60.0;

    #[derive(Debug)]
    struct TempEvent {
        time: f64,
        order: u8,
        kind: TempEventKind,
    }

    #[derive(Debug)]
    enum TempEventKind {
        NoteOn { channel: u8, note: u8, velocity: u8 },
        NoteOff { channel: u8, note: u8 },
        ControlChange { channel: u8, cc: u8, value: u8 },
        ProgramChange { channel: u8, program: u8 },
    }

    let mut temp_events = Vec::new();

    for event in events {
        temp_events.push(TempEvent {
            time: event.time(),
            order: event.timing_order(),
            kind: match event {
                MidiEvent::ControlChange {
                    channel, cc, value, ..
                } => TempEventKind::ControlChange {
                    channel: *channel,
                    cc: *cc,
                    value: *value,
                },
                MidiEvent::ProgramChange {
                    channel, program, ..
                } => TempEventKind::ProgramChange {
                    channel: *channel,
                    program: *program,
                },
                MidiEvent::Note { .. } => continue,
            },
        });
    }

    for event in events {
        if let MidiEvent::Note {
            time,
            duration,
            channel,
            note,
            velocity,
        } = event
        {
            temp_events.push(TempEvent {
                time: *time,
                order: 10,
                kind: TempEventKind::NoteOn {
                    channel: *channel,
                    note: *note,
                    velocity: *velocity,
                },
            });
            temp_events.push(TempEvent {
                time: *time + *duration,
                order: 11,
                kind: TempEventKind::NoteOff {
                    channel: *channel,
                    note: *note,
                },
            });
        }
    }

    temp_events.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap()
            .then_with(|| a.order.cmp(&b.order))
    });

    let mut track = Vec::new();
    let mut current_time = 0.0;
    // let mut current_ticks = 0;

    for event in temp_events {
        let delta_time = event.time - current_time;
        // Ensure delta_time is non-negative (floating point precision issues?)
        let delta_time = delta_time.max(0.0);
        let delta_ticks = (delta_time * ticks_per_second).round() as u32;

        match event.kind {
            TempEventKind::NoteOn {
                channel,
                note,
                velocity,
            } => {
                track.push(TrackEvent {
                    delta: delta_ticks.into(),
                    kind: midly::TrackEventKind::Midi {
                        channel: channel.into(),
                        message: MidiMessage::NoteOn {
                            key: note.into(),
                            vel: velocity.into(),
                        },
                    },
                });
            }
            TempEventKind::NoteOff { channel, note } => {
                track.push(TrackEvent {
                    delta: delta_ticks.into(),
                    kind: midly::TrackEventKind::Midi {
                        channel: channel.into(),
                        message: MidiMessage::NoteOff {
                            key: note.into(),
                            vel: 0.into(),
                        },
                    },
                });
            }
            TempEventKind::ControlChange { channel, cc, value } => {
                track.push(TrackEvent {
                    delta: delta_ticks.into(),
                    kind: midly::TrackEventKind::Midi {
                        channel: channel.into(),
                        message: MidiMessage::Controller {
                            controller: cc.into(),
                            value: value.into(),
                        },
                    },
                });
            }
            TempEventKind::ProgramChange { channel, program } => {
                track.push(TrackEvent {
                    delta: delta_ticks.into(),
                    kind: midly::TrackEventKind::Midi {
                        channel: channel.into(),
                        message: MidiMessage::ProgramChange {
                            program: program.into(),
                        },
                    },
                });
            }
        }

        current_time = event.time;
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
