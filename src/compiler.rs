use crate::dsl::token::{Bar, Block, Note, Song, Token, Track};
use miette::{Diagnostic, Result};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum CompileError {
    #[error("Compilation error: {0}")]
    #[diagnostic(code(loom::compiler::base))]
    Base(String),
}

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
    pub fn new(song: &Song) -> Result<Self> {
        // Simple logic: signature 4/4 -> 4 beats per bar.
        // If unit is bar, block = 4.0.
        // If unit is beat, block = 1.0.
        let sig_parts: Vec<&str> = song.metadata.signature.split('/').collect();
        let num: f64 = sig_parts
            .first()
            .unwrap_or(&"4")
            .parse()
            .map_err(|_| CompileError::Base("Invalid signature numerator".to_string()))?;

        let unit_per_block = if song.metadata.unit == "beat" {
            1.0
        } else {
            // "bar"
            num
        };

        Ok(Self { unit_per_block })
    }

    pub fn compile(&self, song: &Song) -> Result<Vec<MidiEvent>> {
        let mut events = Vec::new();

        for track in &song.tracks {
            self.compile_track(track, &mut events, song.metadata.pitch)?;
        }

        // Sort events by time
        events.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());

        Ok(events)
    }

    fn compile_track(
        &self,
        track: &Track,
        events: &mut Vec<MidiEvent>,
        pitch_offset: i32,
    ) -> Result<()> {
        let mut section_start_time = 0.0;

        for section in &track.sections {
            let mut section_end_time = section_start_time;

            for line in &section.lines {
                let mut current_time = section_start_time;
                // Last note event index PER note in the chord
                let last_event_indices: Vec<Option<usize>> = vec![None; line.notes.len()];

                let mut line_compiler = LineCompiler {
                    channel: track.channel,
                    notes: &line.notes,
                    events,
                    last_event_indices,
                    pitch_offset,
                };

                let mut repeat_buffer: Vec<&Block> = Vec::new();
                let mut in_repeat = false;

                for block in &line.blocks {
                    // Check Start Bar
                    match block.start_bar {
                        Bar::RepeatStart => {
                            repeat_buffer.clear();
                            in_repeat = true;
                        }
                        Bar::RepeatEnd => {
                            if in_repeat {
                                for buffered_block in &repeat_buffer {
                                    let block_duration = self.unit_per_block;
                                    line_compiler.process_tokens(
                                        &buffered_block.tokens,
                                        current_time,
                                        block_duration,
                                    );
                                    current_time += block_duration;
                                }
                                repeat_buffer.clear();
                                in_repeat = false;
                            }
                        }
                        Bar::Double => {
                            if in_repeat {
                                for buffered_block in &repeat_buffer {
                                    let block_duration = self.unit_per_block;
                                    line_compiler.process_tokens(
                                        &buffered_block.tokens,
                                        current_time,
                                        block_duration,
                                    );
                                    current_time += block_duration;
                                }
                                repeat_buffer.clear();
                            }
                            in_repeat = true;
                        }
                        Bar::Standard => {}
                    }

                    // Process Current Block
                    let block_duration = self.unit_per_block;
                    line_compiler.process_tokens(&block.tokens, current_time, block_duration);
                    current_time += block_duration;

                    // Buffer Current Block
                    if in_repeat {
                        repeat_buffer.push(block);
                    }
                }

                // Handle Line End Bar
                match line.end_bar {
                    Bar::RepeatEnd | Bar::Double => {
                        if in_repeat {
                            for buffered_block in &repeat_buffer {
                                let block_duration = self.unit_per_block;
                                line_compiler.process_tokens(
                                    &buffered_block.tokens,
                                    current_time,
                                    block_duration,
                                );
                                current_time += block_duration;
                            }
                        }
                    }
                    _ => {}
                }

                if current_time > section_end_time {
                    section_end_time = current_time;
                }
            }
            section_start_time = section_end_time;
        }
        Ok(())
    }
}

struct LineCompiler<'a> {
    channel: u8,
    notes: &'a [Note],
    events: &'a mut Vec<MidiEvent>,
    last_event_indices: Vec<Option<usize>>,
    pitch_offset: i32,
}

impl<'a> LineCompiler<'a> {
    fn process_tokens(&mut self, tokens: &[Token], start_time: f64, total_duration: f64) {
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
                    for (nth, note) in self.notes.iter().enumerate() {
                        let midi_val = match note {
                            Note::Pitch { .. } => {
                                (note.to_midi() as i32 + self.pitch_offset).clamp(0, 127) as u8
                            }
                            Note::Drum(_) => note.to_midi(),
                        };
                        let event = MidiEvent {
                            time: token_time,
                            duration: duration_per_token,
                            channel: self.channel - 1,
                            note: midi_val,
                            velocity: 100,
                        };
                        self.events.push(event);
                        self.last_event_indices[nth] = Some(self.events.len() - 1);
                    }
                }
                Token::Rest => {
                    for (nth, _) in self.notes.iter().enumerate() {
                        self.last_event_indices[nth] = None;
                    }
                }
                Token::Sustain => {
                    for (nth, _) in self.notes.iter().enumerate() {
                        if let Some(idx) = self.last_event_indices[nth] {
                            self.events[idx].duration += duration_per_token;
                        }
                    }
                }
                Token::Group(sub_tokens) => {
                    self.process_tokens(sub_tokens, token_time, duration_per_token);
                }
            }
        }
    }
}
