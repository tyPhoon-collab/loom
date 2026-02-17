use crate::token::{Song, Token, Track};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct MidiEvent {
    pub time: f64,     // Absolute time in beats
    pub duration: f64, // Duration in beats
    pub channel: u8,
    pub note: String,
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
            // Per line state (Last note event index for sustain)
            // Note: Parallel lines (polyphony) are independent rows usually.
            // But if multiple lines share the same note?
            // "Note" in Line is just "row header".
            // So sustain logic is strictly Per Row.

            let mut current_time = 0.0;
            let mut last_event_idx: Option<usize> = None;
            let note_key = &line.note;

            for block in &line.blocks {
                let block_duration = self.unit_per_block;
                self.process_tokens(
                    &block.tokens,
                    current_time,
                    block_duration,
                    track.channel,
                    note_key,
                    events,
                    &mut last_event_idx,
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
        note_key: &str,
        events: &mut Vec<MidiEvent>,
        last_event_idx: &mut Option<usize>,
    ) {
        if tokens.is_empty() {
            // Empty block just advances time (done by caller)
            // But sustain state? If empty, does it sustain?
            // CONCEPT.md says "Space ... visual only".
            // Empty tokens means NO events.
            // If implicit sustain is desired, one must write `-`.
            // So empty list -> do nothing.
            return;
        }

        let len = tokens.len() as f64;
        let duration_per_token = total_duration / len;

        for (i, token) in tokens.iter().enumerate() {
            let token_time = start_time + (i as f64 * duration_per_token);

            match token {
                Token::Note => {
                    // New Event
                    let event = MidiEvent {
                        time: token_time,
                        duration: duration_per_token, // Default duration
                        channel,
                        note: note_key.to_string(),
                        velocity: 100,
                    };
                    events.push(event);
                    *last_event_idx = Some(events.len() - 1);
                }
                Token::Rest => {
                    // Stop sustaining
                    *last_event_idx = None;
                }
                Token::Sustain => {
                    // Extend previous
                    if let Some(idx) = *last_event_idx {
                        // Mutate the event inside the vector
                        if let Some(event) = events.get_mut(idx) {
                            event.duration += duration_per_token;
                        }
                    } else {
                        // No previous note to sustain. Ignore or warn?
                        // Ignore for now.
                    }
                }
                Token::Group(sub_tokens) => {
                    // Recursive
                    self.process_tokens(
                        sub_tokens,
                        token_time,
                        duration_per_token,
                        channel,
                        note_key,
                        events,
                        last_event_idx,
                    );
                }
            }
        }
    }
}
