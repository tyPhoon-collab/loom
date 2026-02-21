use crate::dsl::token::{Bar, Block, ModifierKind, ModifierValue, Note, Song, Token, Track};
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

/// Resolved modifier values for a line, flattened per block
struct ResolvedModifiers {
    velocities: Vec<Vec<i32>>, // per block, per token
    pitches: Vec<Vec<i32>>,    // per block, per token
}

impl Compiler {
    pub fn new(song: &Song) -> Result<Self> {
        let sig_parts: Vec<&str> = song.metadata.signature.split('/').collect();
        let num: f64 = sig_parts
            .first()
            .unwrap_or(&"4")
            .parse()
            .map_err(|_| CompileError::Base("Invalid signature numerator".to_string()))?;

        let unit_per_block = if song.metadata.unit == "beat" {
            1.0
        } else {
            num
        };

        Ok(Self { unit_per_block })
    }

    pub fn compile(&self, song: &Song) -> Result<Vec<MidiEvent>> {
        let mut events = Vec::new();

        for track in &song.tracks {
            self.compile_track(track, &mut events, song.metadata.pitch)?;
        }

        events.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());

        Ok(events)
    }

    /// Resolve modifier values for a line into per-block, per-token arrays
    fn resolve_modifiers(&self, line: &crate::dsl::token::Line) -> ResolvedModifiers {
        let num_blocks = line.blocks.len();

        // Count tokens per block
        let tokens_per_block: Vec<usize> = line.blocks.iter().map(|b| b.tokens.len()).collect();

        // Initialize with defaults
        let mut velocities: Vec<Vec<i32>> = tokens_per_block
            .iter()
            .map(|&n| vec![ModifierKind::Velocity.default_value(); n])
            .collect();
        let mut pitches: Vec<Vec<i32>> = tokens_per_block
            .iter()
            .map(|&n| vec![ModifierKind::Pitch.default_value(); n])
            .collect();

        for modifier in &line.modifiers {
            let default_val = modifier.kind.default_value();
            let target = match modifier.kind {
                ModifierKind::Velocity => &mut velocities,
                ModifierKind::Pitch => &mut pitches,
            };

            let mut latch_value: Option<i32> = None;
            let mut mod_block_idx = 0;

            for block_idx in 0..num_blocks {
                let num_tokens = tokens_per_block[block_idx];
                if num_tokens == 0 {
                    continue;
                }

                // Get modifier values for this block (if available)
                let mod_values = if mod_block_idx < modifier.blocks.len() {
                    mod_block_idx += 1;
                    &modifier.blocks[mod_block_idx - 1].values
                } else {
                    &vec![]
                };

                for (token_idx, target_slot) in
                    target[block_idx].iter_mut().enumerate().take(num_tokens)
                {
                    let mod_val = mod_values.get(token_idx).and_then(|v| v.as_ref());
                    match mod_val {
                        Some(ModifierValue::Set(v)) => {
                            *target_slot = *v;
                            latch_value = None; // One-shot: clear latch
                        }
                        Some(ModifierValue::Latch(v)) => {
                            *target_slot = *v;
                            latch_value = Some(*v);
                        }
                        None => {
                            // If latched, continue with latch value; otherwise default
                            *target_slot = latch_value.unwrap_or(default_val);
                        }
                    }
                }
            }
        }

        ResolvedModifiers {
            velocities,
            pitches,
        }
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
                let last_event_indices: Vec<Option<usize>> = vec![None; line.notes.len()];

                let resolved = self.resolve_modifiers(line);

                let mut line_compiler = LineCompiler {
                    channel: track.channel,
                    notes: &line.notes,
                    events,
                    last_event_indices,
                    pitch_offset,
                    resolved_velocities: &resolved.velocities,
                    resolved_pitches: &resolved.pitches,
                    current_block_idx: 0,
                };

                let mut repeat_buffer: Vec<(usize, &Block)> = Vec::new();
                let mut in_repeat = false;

                for (block_idx, block) in line.blocks.iter().enumerate() {
                    match block.start_bar {
                        Bar::RepeatStart => {
                            repeat_buffer.clear();
                            in_repeat = true;
                        }
                        Bar::RepeatEnd => {
                            if in_repeat {
                                for &(buf_idx, buffered_block) in &repeat_buffer {
                                    let block_duration = self.unit_per_block;
                                    line_compiler.current_block_idx = buf_idx;
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
                                for &(buf_idx, buffered_block) in &repeat_buffer {
                                    let block_duration = self.unit_per_block;
                                    line_compiler.current_block_idx = buf_idx;
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

                    let block_duration = self.unit_per_block;
                    line_compiler.current_block_idx = block_idx;
                    line_compiler.process_tokens(&block.tokens, current_time, block_duration);
                    current_time += block_duration;

                    if in_repeat {
                        repeat_buffer.push((block_idx, block));
                    }
                }

                match line.end_bar {
                    Bar::RepeatEnd | Bar::Double => {
                        if in_repeat {
                            for &(buf_idx, buffered_block) in &repeat_buffer {
                                let block_duration = self.unit_per_block;
                                line_compiler.current_block_idx = buf_idx;
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
    resolved_velocities: &'a [Vec<i32>],
    resolved_pitches: &'a [Vec<i32>],
    current_block_idx: usize,
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
                    // Get velocity and pitch modifier for this token
                    let velocity = self
                        .resolved_velocities
                        .get(self.current_block_idx)
                        .and_then(|v| v.get(i))
                        .copied()
                        .unwrap_or(100)
                        .clamp(0, 127) as u8;

                    let pitch_mod = self
                        .resolved_pitches
                        .get(self.current_block_idx)
                        .and_then(|v| v.get(i))
                        .copied()
                        .unwrap_or(0);

                    for (nth, note) in self.notes.iter().enumerate() {
                        let midi_val = match note {
                            Note::Pitch { .. } => {
                                (note.to_midi() as i32 + self.pitch_offset + pitch_mod)
                                    .clamp(0, 127) as u8
                            }
                            Note::Drum(_) => {
                                (note.to_midi() as i32 + pitch_mod).clamp(0, 127) as u8
                            }
                        };
                        let event = MidiEvent {
                            time: token_time,
                            duration: duration_per_token,
                            channel: self.channel - 1,
                            note: midi_val,
                            velocity,
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
