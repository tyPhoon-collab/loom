use super::{
    CompileError, CompileResult, InvalidModifierStructureData, MidiEvent, ResolvedModifierValue,
};
use crate::compiler::swing::apply_swing_to_time;
use crate::dsl::token::{ModifierKind, Note, Token};

pub(super) struct LineCompiler<'a> {
    pub(super) channel: u8,
    pub(super) notes: &'a [Note],
    pub(super) events: &'a mut Vec<MidiEvent>,
    pub(super) last_event_indices: Vec<usize>,
    pub(super) pitch_offset: i32,
    pub(super) resolved_velocities: &'a [Vec<ResolvedModifierValue>],
    pub(super) resolved_pitches: &'a [Vec<ResolvedModifierValue>],
    pub(super) track_name: &'a str,
    pub(super) context: &'a str,
    pub(super) current_block_idx: usize,
    pub(super) leaf_counter: usize,
    pub(super) swing: Option<(u8, u8)>,
}

impl<'a> LineCompiler<'a> {
    fn emit_note_set(
        &mut self,
        note_set: &[Note],
        token_time: f64,
        swung_duration: f64,
        leaf_idx: usize,
    ) -> CompileResult<()> {
        let velocity_values = self
            .resolved_velocities
            .get(self.current_block_idx)
            .and_then(|v| v.get(leaf_idx))
            .cloned()
            .unwrap_or(ResolvedModifierValue::Scalar(100));

        let pitch_values = self
            .resolved_pitches
            .get(self.current_block_idx)
            .and_then(|v| v.get(leaf_idx))
            .cloned()
            .unwrap_or(ResolvedModifierValue::Scalar(0));

        let velocity_at = |idx: usize| -> CompileResult<i32> {
            match &velocity_values {
                ResolvedModifierValue::Scalar(v) => Ok(*v),
                ResolvedModifierValue::PerNote(values) => {
                    values.get(idx).copied().ok_or_else(|| {
                        CompileError::InvalidModifierStructure(Box::new(
                            InvalidModifierStructureData {
                                track: self.track_name.to_string(),
                                context: self.context.to_string(),
                                modifier: ModifierKind::Velocity.to_string(),
                                block_index: self.current_block_idx,
                                value_path: format!("{}", leaf_idx),
                                reason: format!(
                                    "note-list length {} does not match note count {}",
                                    values.len(),
                                    note_set.len()
                                ),
                            },
                        ))
                    })
                }
            }
        };

        let pitch_at = |idx: usize| -> CompileResult<i32> {
            match &pitch_values {
                ResolvedModifierValue::Scalar(v) => Ok(*v),
                ResolvedModifierValue::PerNote(values) => {
                    values.get(idx).copied().ok_or_else(|| {
                        CompileError::InvalidModifierStructure(Box::new(
                            InvalidModifierStructureData {
                                track: self.track_name.to_string(),
                                context: self.context.to_string(),
                                modifier: ModifierKind::Pitch.to_string(),
                                block_index: self.current_block_idx,
                                value_path: format!("{}", leaf_idx),
                                reason: format!(
                                    "note-list length {} does not match note count {}",
                                    values.len(),
                                    note_set.len()
                                ),
                            },
                        ))
                    })
                }
            }
        };

        if let ResolvedModifierValue::PerNote(values) = &velocity_values {
            if values.len() != note_set.len() {
                return Err(CompileError::InvalidModifierStructure(Box::new(
                    InvalidModifierStructureData {
                        track: self.track_name.to_string(),
                        context: self.context.to_string(),
                        modifier: ModifierKind::Velocity.to_string(),
                        block_index: self.current_block_idx,
                        value_path: format!("{}", leaf_idx),
                        reason: format!(
                            "note-list length {} does not match note count {}",
                            values.len(),
                            note_set.len()
                        ),
                    },
                )));
            }
        }
        if let ResolvedModifierValue::PerNote(values) = &pitch_values {
            if values.len() != note_set.len() {
                return Err(CompileError::InvalidModifierStructure(Box::new(
                    InvalidModifierStructureData {
                        track: self.track_name.to_string(),
                        context: self.context.to_string(),
                        modifier: ModifierKind::Pitch.to_string(),
                        block_index: self.current_block_idx,
                        value_path: format!("{}", leaf_idx),
                        reason: format!(
                            "note-list length {} does not match note count {}",
                            values.len(),
                            note_set.len()
                        ),
                    },
                )));
            }
        }

        let velocity = velocity_at(0)?;
        let velocity = crate::validation::ensure_u7_i32(velocity, "Velocity").map_err(|_| {
            CompileError::VelocityOutOfRange {
                track: self.track_name.to_string(),
                context: self.context.to_string(),
                block_index: self.current_block_idx,
                leaf_index: leaf_idx,
                value: velocity,
            }
        })?;

        self.last_event_indices.clear();
        for (note_idx, note) in note_set.iter().enumerate() {
            let velocity = match &velocity_values {
                ResolvedModifierValue::Scalar(_) => velocity,
                ResolvedModifierValue::PerNote(_) => {
                    let raw_velocity = velocity_at(note_idx)?;
                    crate::validation::ensure_u7_i32(raw_velocity, "Velocity").map_err(|_| {
                        CompileError::VelocityOutOfRange {
                            track: self.track_name.to_string(),
                            context: self.context.to_string(),
                            block_index: self.current_block_idx,
                            leaf_index: leaf_idx,
                            value: raw_velocity,
                        }
                    })?
                }
            };
            let pitch_mod = pitch_at(note_idx)?;
            let base_note = note
                .to_midi_checked()
                .map_err(|e| CompileError::InvalidNote {
                    track: self.track_name.to_string(),
                    context: self.context.to_string(),
                    block_index: self.current_block_idx,
                    leaf_index: leaf_idx,
                    note: note.to_string(),
                    reason: e.to_string(),
                })? as i32;
            let midi_val = match note {
                Note::Pitch { .. } | Note::Midi(_) => crate::validation::ensure_u7_i32(
                    base_note + self.pitch_offset + pitch_mod,
                    "MIDI note",
                )
                .map_err(|_| CompileError::NoteOutOfRange {
                    track: self.track_name.to_string(),
                    context: self.context.to_string(),
                    block_index: self.current_block_idx,
                    leaf_index: leaf_idx,
                    note: note.to_string(),
                    value: base_note + self.pitch_offset + pitch_mod,
                })?,
                Note::Drum(_) => {
                    crate::validation::ensure_u7_i32(base_note + pitch_mod, "MIDI note").map_err(
                        |_| CompileError::NoteOutOfRange {
                            track: self.track_name.to_string(),
                            context: self.context.to_string(),
                            block_index: self.current_block_idx,
                            leaf_index: leaf_idx,
                            note: note.to_string(),
                            value: base_note + pitch_mod,
                        },
                    )?
                }
            };
            self.events.push(MidiEvent::Note {
                time: token_time,
                duration: swung_duration,
                channel: self.channel,
                note: midi_val,
                velocity,
            });
            self.last_event_indices.push(self.events.len() - 1);
        }
        Ok(())
    }

    pub(super) fn process_tokens(
        &mut self,
        tokens: &[Token],
        start_time: f64,
        total_duration: f64,
    ) -> CompileResult<()> {
        if tokens.is_empty() {
            return Ok(());
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
                    let leaf_idx = self.leaf_counter;
                    self.emit_note_set(self.notes, token_time, swung_duration, leaf_idx)?;
                    self.leaf_counter += 1;
                }
                Token::NoteLiteral(note_set) => {
                    let leaf_idx = self.leaf_counter;
                    self.emit_note_set(note_set, token_time, swung_duration, leaf_idx)?;
                    self.leaf_counter += 1;
                }
                Token::Rest => {
                    self.last_event_indices.clear();
                    self.leaf_counter += 1;
                }
                Token::Sustain => {
                    for idx in &self.last_event_indices {
                        if let MidiEvent::Note { duration, .. } = &mut self.events[*idx] {
                            *duration += swung_duration;
                        }
                    }
                    self.leaf_counter += 1;
                }
                Token::Group(sub_tokens) => {
                    self.process_tokens(sub_tokens, unswung_start, duration_per_token)?;
                }
            }
        }
        Ok(())
    }
}
