use crate::token::{Note, Song, Token, Track};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct MidiEvent {
    pub time: f64,     // Absolute time in beats
    pub duration: f64, // Duration in beats
    pub channel: u8,
    pub note: u8,
    pub velocity: u8,
}

pub struct Compiler {
    pub unit_per_block: f64, // e.g. 4.0 for 4/4 bar
}

impl Compiler {
    pub fn new(song: &Song) -> Self {
        // Simple logic: signature 4/4 -> 4 beats per bar.
        // If unit is bar, block = 4.0.
        // If unit is beat, block = 1.0.
        let sig_parts: Vec<&str> = song.metadata.signature.split('/').collect();
        let num: f64 = sig_parts.first().unwrap_or(&"4").parse().unwrap_or(4.0);
        // let denom: f64 = sig_parts.get(1).unwrap_or(&"4").parse().unwrap_or(4.0);

        let unit_per_block = if song.metadata.unit == "beat" {
            1.0
        } else {
            // "bar"
            num // In 4/4, a bar is 4 beats. In 3/4, 3 beats.
        };

        Self { unit_per_block }
    }

    pub fn compile(&self, song: &Song) -> Result<Vec<MidiEvent>> {
        let mut events = Vec::new();

        for track in &song.tracks {
            self.compile_track(track, &mut events)?;
        }

        // Sort events by time
        events.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());

        Ok(events)
    }

    fn compile_track(&self, track: &Track, events: &mut Vec<MidiEvent>) -> Result<()> {
        for line in &track.lines {
            let mut current_time = 0.0;
            // Last note event index PER note in the chord
            let mut last_event_indices: Vec<Option<usize>> = vec![None; line.notes.len()];

            for block in &line.blocks {
                let block_duration = self.unit_per_block;
                self.process_tokens(
                    &block.tokens,
                    current_time,
                    block_duration,
                    track.channel,
                    &line.notes,
                    events,
                    &mut last_event_indices,
                );
                current_time += block_duration;
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn process_tokens(
        &self,
        tokens: &[Token],
        start_time: f64,
        total_duration: f64,
        channel: u8,
        notes: &[Note],
        events: &mut Vec<MidiEvent>,
        last_event_indices: &mut Vec<Option<usize>>,
    ) {
        if tokens.is_empty() {
            return;
        }

        let len = tokens.len() as f64;
        let duration_per_token = total_duration / len;

        for (i, token) in tokens.iter().enumerate() {
            let token_time = start_time + (i as f64 * duration_per_token);

            match token {
                Token::Note => {
                    // Start new events for ALL notes in the chord
                    for (nth, note) in notes.iter().enumerate() {
                        let event = MidiEvent {
                            time: token_time,
                            duration: duration_per_token,
                            channel,
                            note: note.to_midi(),
                            velocity: 100,
                        };
                        events.push(event);
                        last_event_indices[nth] = Some(events.len() - 1);
                    }
                }
                Token::Rest => {
                    // Stop sustaining ALL notes in the chord
                    for idx in last_event_indices.iter_mut() {
                        *idx = None;
                    }
                }
                Token::Sustain => {
                    // Extend previous events for ALL notes in the chord
                    for idx_opt in last_event_indices.iter().flatten() {
                        if let Some(event) = events.get_mut(*idx_opt) {
                            event.duration += duration_per_token;
                        }
                    }
                }
                Token::Group(sub_tokens) => {
                    self.process_tokens(
                        sub_tokens,
                        token_time,
                        duration_per_token,
                        channel,
                        notes,
                        events,
                        last_event_indices,
                    );
                }
            }
        }
    }
}
