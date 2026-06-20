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
    if !song.fragment_blocks.is_empty() {
        return collect_fragment_track_events(song);
    }

    let mut out = Vec::new();
    let solo_active = song.tracks.iter().any(|track| track.solo);

    for track in &song.tracks {
        if track.muted || (solo_active && !track.solo) {
            continue;
        }
        let single_song = Song {
            metadata: song.metadata.clone(),
            tracks: vec![track.clone()],
            templates: song.templates.clone(),
            fragment_blocks: Vec::new(),
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

fn collect_fragment_track_events(song: &Song) -> Result<Vec<TrackEvent>> {
    let compiler_inst = compiler::Compiler::new(song)?;
    let events = compiler_inst
        .compile(song)
        .map_err(|e| miette!("Compiler error: {}", e))?;
    let track_by_zero_based_channel: std::collections::HashMap<u8, String> = song
        .tracks
        .iter()
        .filter_map(|track| {
            crate::validation::to_zero_based_channel(track.channel)
                .ok()
                .map(|channel| (channel, track.name.clone()))
        })
        .collect();

    Ok(events
        .into_iter()
        .map(|event| {
            let track = track_by_zero_based_channel
                .get(&event.channel())
                .cloned()
                .unwrap_or_else(|| format!("Channel {}", event.channel() + 1));
            TrackEvent { track, event }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::collect_track_events;
    use crate::dsl::parser::parse_song;

    #[test]
    fn collect_track_events_respects_solo_filter() {
        let song = parse_song(
            "# Piano: 1 s\nC4 | ^ |\n\n# Bass: 2\nC2 | ^ |\n\n# Lead: 3 s x\nE4 | ^ |\n"
                .to_string(),
        )
        .unwrap();
        let events = collect_track_events(&song).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].track, "Piano");
    }
}
