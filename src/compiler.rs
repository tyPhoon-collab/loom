use crate::dsl::token::{
    Bar, Block, Line, LineEntry, ModifierKind, ModifierValue, Note, Song, TemplateParam, Token,
    Track,
};
use miette::{Diagnostic, Result};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum CompileError {
    #[error("Compilation error: {0}")]
    #[diagnostic(code(loom::compiler::base))]
    Base(String),

    #[error("Circular template reference detected: {0}")]
    #[diagnostic(code(loom::compiler::circular_template_reference))]
    CircularTemplateReference(String),
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

struct CompilerContext<'a> {
    events: &'a mut Vec<MidiEvent>,
    templates: &'a std::collections::HashMap<String, crate::dsl::token::TemplateDef>,
    call_stack: &'a mut Vec<String>,
    swing: Option<(u8, u8)>,
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
            if track.muted {
                continue;
            }
            self.compile_track(
                track,
                &mut events,
                song.metadata.pitch,
                &song.templates,
                song.metadata.swing.values(),
            )?;
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
                    let mod_val = mod_values.get(token_idx).unwrap_or(&ModifierValue::Empty);
                    match mod_val {
                        ModifierValue::Set(v) => {
                            *target_slot = *v;
                            latch_value = None; // One-shot: clear latch
                        }
                        ModifierValue::Latch(v) => {
                            *target_slot = *v;
                            latch_value = Some(*v);
                        }
                        ModifierValue::Empty => {
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
        templates: &std::collections::HashMap<String, crate::dsl::token::TemplateDef>,
        swing: Option<(u8, u8)>,
    ) -> Result<()> {
        let mut section_start_time = 0.0;
        let mut section_max_time = 0.0;
        let mut call_stack = Vec::new();
        let mut ctx = CompilerContext {
            events,
            templates,
            call_stack: &mut call_stack,
            swing,
        };

        for entry in &track.sequence.entries {
            match entry {
                LineEntry::Pattern(line) => {
                    let mut line_time = section_start_time;
                    self.compile_pattern_line(
                        line,
                        track.channel,
                        ctx.events,
                        pitch_offset,
                        &mut line_time,
                        ctx.swing,
                    );
                    if line_time > section_max_time {
                        section_max_time = line_time;
                    }
                }
                LineEntry::TemplateCalls(sub_calls) => {
                    for call in sub_calls {
                        let mut line_time = section_start_time;
                        self.compile_template(
                            call,
                            track.channel,
                            pitch_offset,
                            &mut line_time,
                            &mut ctx,
                        )?;
                        section_max_time = section_max_time.max(line_time);
                    }
                }
                LineEntry::TrackWrap => {
                    section_start_time = section_max_time;
                }
            }
        }
        Ok(())
    }

    fn compile_pattern_line(
        &self,
        line: &Line,
        channel: u8,
        events: &mut Vec<MidiEvent>,
        pitch_offset: i32,
        current_time: &mut f64,
        swing: Option<(u8, u8)>,
    ) {
        let initial_time = *current_time;
        let mut line_time = initial_time;
        let last_event_indices: Vec<Option<usize>> = vec![None; line.notes.len()];

        let resolved = self.resolve_modifiers(line);

        let mut line_compiler = LineCompiler {
            channel,
            notes: &line.notes,
            events,
            last_event_indices,
            pitch_offset,
            resolved_velocities: &resolved.velocities,
            resolved_pitches: &resolved.pitches,
            current_block_idx: 0,
            swing,
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
                                line_time,
                                block_duration,
                            );
                            line_time += block_duration;
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
                                line_time,
                                block_duration,
                            );
                            line_time += block_duration;
                        }
                        repeat_buffer.clear();
                    }
                    in_repeat = true;
                }
                Bar::Standard => {}
            }

            let block_duration = self.unit_per_block;
            line_compiler.current_block_idx = block_idx;
            line_compiler.process_tokens(&block.tokens, line_time, block_duration);
            line_time += block_duration;

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
                            line_time,
                            block_duration,
                        );
                        line_time += block_duration;
                    }
                }
            }
            _ => {}
        }

        *current_time = line_time;
    }

    fn compile_template(
        &self,
        call: &crate::dsl::token::TemplateCall,
        channel: u8,
        mut pitch_offset: i32,
        current_time: &mut f64,
        ctx: &mut CompilerContext,
    ) -> Result<()> {
        if ctx.call_stack.contains(&call.name) {
            let trace = ctx.call_stack.join(" -> ") + " -> " + &call.name;
            return Err(CompileError::CircularTemplateReference(trace).into());
        }
        ctx.call_stack.push(call.name.clone());
        let def = ctx
            .templates
            .get(&call.name)
            .ok_or_else(|| CompileError::Base(format!("Template not found: {}", call.name)))?;

        let mut template_pitch_offset = 0;
        let mut structural_repeat = 1u32;
        let mut reverse = false;

        for param in &call.params {
            match param {
                TemplateParam::Transpose(v) => template_pitch_offset += v,
                TemplateParam::StructuralRepeat(v) => structural_repeat = *v,
                TemplateParam::Macro(m) if m == "rev" => reverse = true,
                _ => {}
            }
        }

        pitch_offset += template_pitch_offset;

        for _ in 0..call.repeat {
            let mut entries = def.sequence.entries.clone();
            if reverse {
                entries.reverse();
            }

            let mut section_start_time = *current_time;
            let mut section_max_time = section_start_time;

            for entry in &entries {
                match entry {
                    LineEntry::Pattern(line) => {
                        let mut line_repeated = line.clone();
                        if structural_repeat > 1 {
                            for block in &mut line_repeated.blocks {
                                let original_tokens = block.tokens.clone();
                                block.tokens.clear();
                                for _ in 0..structural_repeat {
                                    block.tokens.extend(original_tokens.clone());
                                }
                            }
                        }
                        let mut line_time = section_start_time;
                        self.compile_pattern_line(
                            &line_repeated,
                            channel,
                            ctx.events,
                            pitch_offset,
                            &mut line_time,
                            ctx.swing,
                        );
                        if line_time > section_max_time {
                            section_max_time = line_time;
                        }
                    }
                    LineEntry::TemplateCalls(sub_calls) => {
                        for call in sub_calls {
                            let mut line_time = section_start_time;
                            self.compile_template(
                                call,
                                channel,
                                pitch_offset,
                                &mut line_time,
                                ctx,
                            )?;
                            section_max_time = section_max_time.max(line_time);
                        }
                    }
                    LineEntry::TrackWrap => {
                        section_start_time = section_max_time;
                    }
                }
            }
            *current_time = section_max_time;
        }

        ctx.call_stack.pop();
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
    swing: Option<(u8, u8)>,
}

fn apply_swing_to_time(time: f64, swing: Option<(u8, u8)>) -> f64 {
    let (swing_grid, swing_amount) = match swing {
        Some((g, a)) => (g, a),
        None => return time,
    };
    if swing_grid == 0 || swing_amount == 50 {
        return time;
    }
    let grid = 4.0 / (swing_grid as f64);
    let pair_cycle = grid * 2.0;

    let time_with_eps = time + 1e-9;
    let cycle_start = (time_with_eps / pair_cycle).floor() * pair_cycle;
    let pos_in_cycle = time - cycle_start;

    let first_half_duration = pair_cycle * (swing_amount as f64) / 100.0;
    let first_half_ratio = first_half_duration / grid;

    if pos_in_cycle < grid - 1e-9 {
        cycle_start + pos_in_cycle * first_half_ratio
    } else {
        let second_half_duration = pair_cycle - first_half_duration;
        let second_half_ratio = second_half_duration / grid;
        let pos_in_second_half = pos_in_cycle - grid;
        cycle_start + first_half_duration + pos_in_second_half * second_half_ratio
    }
}

impl<'a> LineCompiler<'a> {
    fn process_tokens(&mut self, tokens: &[Token], start_time: f64, total_duration: f64) {
        if tokens.is_empty() {
            return;
        }

        let len = tokens.len() as f64;
        let duration_per_token = total_duration / len;

        for (i, token) in tokens.iter().enumerate() {
            let unswung_start = start_time + (i as f64 * duration_per_token);
            let unswung_end = start_time + ((i + 1) as f64 * duration_per_token);

            let token_time = apply_swing_to_time(unswung_start, self.swing);
            let swung_duration = apply_swing_to_time(unswung_end, self.swing) - token_time;

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
                            duration: swung_duration,
                            channel: self.channel - 1, // Fix: 0-indexed MIDI channel
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
                            self.events[idx].duration += swung_duration;
                        }
                    }
                }
                Token::Group(sub_tokens) => {
                    self.process_tokens(sub_tokens, unswung_start, duration_per_token);
                }
            }
        }
    }
}
