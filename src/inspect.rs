use crate::compiler::{self, MidiEvent};
use crate::dsl::token::Song;
use miette::{miette, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TrackEvent {
    pub track: String,
    pub event: MidiEvent,
}

pub fn collect_track_events(song: &Song) -> Result<Vec<TrackEvent>> {
    let mut out = Vec::new();

    for track in &song.tracks {
        if track.muted {
            continue;
        }
        let single_song = Song {
            metadata: song.metadata.clone(),
            tracks: vec![track.clone()],
            templates: song.templates.clone(),
        };
        let compiler_inst = compiler::Compiler::new(&single_song)?;
        let events = compiler_inst
            .compile(&single_song)
            .map_err(|e| miette!("Compiler error: {}", e))?;

        out.extend(events.into_iter().map(|event| TrackEvent {
            track: track.name.clone(),
            event,
        }));
    }

    Ok(out)
}
