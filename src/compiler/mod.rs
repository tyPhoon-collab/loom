pub mod error;
pub mod event;
pub mod humanize;
mod line;
pub mod modifiers;
pub mod swing;
mod template;

#[path = "macro.rs"]
pub mod macros;

pub use error::{CompileContextExt, CompileError, CompileResult, InvalidModifierStructureData};
pub use event::MidiEvent;

use crate::dsl::token::{
    Bar, Block, FragmentBlock, Line, LineEntry, ModifierKind, ModifierValue, Song, Track,
    TrackInitEvent,
};
use miette::Result;

use humanize::apply_humanize;
use line::LineCompiler;
use modifiers::{
    count_leaf_tokens, expand_modifier_values, ResolvedModifierValue, ResolvedModifiers,
};
use template::{CompilerContext, TemplateScope};

pub struct Compiler {
    pub unit_per_block: f64, // e.g. 4.0 for 4/4 bar
}

impl Compiler {
    pub fn new(song: &Song) -> Result<Self> {
        let sig_parts: Vec<&str> = song.metadata.signature.split('/').collect();
        let numerator_raw = sig_parts.first().copied().unwrap_or("4");
        let num: f64 = sig_parts.first().unwrap_or(&"4").parse().map_err(|_| {
            CompileError::InvalidSignature {
                signature: song.metadata.signature.clone(),
                reason: format!("invalid numerator '{}'", numerator_raw),
            }
        })?;

        let unit_per_block = if song.metadata.unit == "beat" {
            1.0
        } else {
            num
        };

        Ok(Self { unit_per_block })
    }

    pub fn compile(&self, song: &Song) -> Result<Vec<MidiEvent>> {
        let mut events =
            compile_track_init_events(song).with_compile_context("collecting track init events")?;

        if song.fragment_blocks.is_empty() {
            let solo_active = song.tracks.iter().any(|track| track.solo);
            for track in &song.tracks {
                if !track_is_active(track, solo_active) {
                    continue;
                }
                self.compile_track(
                    track,
                    &mut events,
                    song.metadata.pitch,
                    TemplateScope {
                        templates: &song.templates,
                        libraries: &song.libraries,
                    },
                    song.metadata.swing.values(),
                )
                .with_compile_context(format!("track '{}'", track.name))?;
            }
        } else {
            self.compile_fragment_blocks(
                &song.fragment_blocks,
                &song.tracks,
                &mut events,
                song.metadata.pitch,
                song.metadata.swing.values(),
            )?;
        }

        if let Some(humanize) = song.metadata.humanize.values() {
            apply_humanize(&mut events, &humanize);
        }

        events.sort_by(|a, b| {
            a.time()
                .partial_cmp(&b.time())
                .unwrap()
                .then_with(|| a.timing_order().cmp(&b.timing_order()))
        });
        Ok(events)
    }

    /// Resolve modifier values for a line into per-block, per-leaf-token arrays (DFS order)
    fn resolve_modifiers(
        &self,
        line: &crate::dsl::token::Line,
        track_name: &str,
        context: &str,
    ) -> CompileResult<ResolvedModifiers> {
        let num_blocks = line.blocks.len();

        // Count leaf tokens per block (DFS)
        let leaves_per_block: Vec<usize> = line
            .blocks
            .iter()
            .map(|b| count_leaf_tokens(&b.tokens))
            .collect();

        // Initialize with defaults
        let mut velocities: Vec<Vec<ResolvedModifierValue>> = leaves_per_block
            .iter()
            .map(|&n| {
                vec![ResolvedModifierValue::Scalar(ModifierKind::Velocity.default_value()); n]
            })
            .collect();
        let mut pitches: Vec<Vec<ResolvedModifierValue>> = leaves_per_block
            .iter()
            .map(|&n| vec![ResolvedModifierValue::Scalar(ModifierKind::Pitch.default_value()); n])
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
                let num_leaves = leaves_per_block[block_idx];
                let maybe_raw_values = if mod_block_idx < modifier.blocks.len() {
                    let raw_values = &modifier.blocks[mod_block_idx].values;
                    mod_block_idx += 1;
                    Some(raw_values)
                } else {
                    None
                };

                if num_leaves == 0 {
                    // Keep modifier block alignment even when the pattern block is empty.
                    continue;
                }

                // Get modifier values for this block, expanded to leaf level
                let expanded = if let Some(raw_values) = maybe_raw_values {
                    let mut path = Vec::new();
                    expand_modifier_values(
                        &line.blocks[block_idx].tokens,
                        raw_values,
                        track_name,
                        context,
                        modifier.kind,
                        block_idx,
                        &mut path,
                    )?
                } else {
                    vec![ModifierValue::Empty; num_leaves]
                };

                for (leaf_idx, target_slot) in
                    target[block_idx].iter_mut().enumerate().take(num_leaves)
                {
                    let mod_val = expanded.get(leaf_idx).unwrap_or(&ModifierValue::Empty);
                    match mod_val {
                        ModifierValue::Set(v) => {
                            *target_slot = ResolvedModifierValue::Scalar(*v);
                            latch_value = None; // One-shot: clear latch
                        }
                        ModifierValue::Latch(v) => {
                            *target_slot = ResolvedModifierValue::Scalar(*v);
                            latch_value = Some(*v);
                        }
                        ModifierValue::NoteList(vals) => {
                            *target_slot = ResolvedModifierValue::PerNote(vals.clone());
                            latch_value = None;
                        }
                        ModifierValue::Empty | ModifierValue::Group(_) => {
                            // If latched, continue with latch value; otherwise default
                            *target_slot =
                                ResolvedModifierValue::Scalar(latch_value.unwrap_or(default_val));
                        }
                    }
                }
            }
        }

        Ok(ResolvedModifiers {
            velocities,
            pitches,
        })
    }

    fn compile_track(
        &self,
        track: &Track,
        events: &mut Vec<MidiEvent>,
        pitch_offset: i32,
        scope: TemplateScope,
        swing: Option<(u8, u8)>,
    ) -> CompileResult<()> {
        self.compile_track_sequence(track, events, pitch_offset, scope, swing, 0.0)?;
        Ok(())
    }

    fn compile_track_sequence(
        &self,
        track: &Track,
        events: &mut Vec<MidiEvent>,
        pitch_offset: i32,
        scope: TemplateScope,
        swing: Option<(u8, u8)>,
        start_time: f64,
    ) -> CompileResult<f64> {
        let mut section_start_time = start_time;
        let mut section_max_time = start_time;
        let mut call_stack = Vec::new();
        let mut ctx = CompilerContext {
            events,
            call_stack: &mut call_stack,
            swing,
        };

        for entry in &track.sequence.entries {
            match entry {
                LineEntry::Pattern(line) => {
                    let mut line_time = section_start_time;
                    self.compile_pattern_line(
                        line,
                        &track.name,
                        "track sequence",
                        track.channel,
                        ctx.events,
                        pitch_offset,
                        &mut line_time,
                        ctx.swing,
                        1.0,
                    )
                    .with_compile_context("pattern line in track sequence")?;
                    if line_time > section_max_time {
                        section_max_time = line_time;
                    }
                }
                LineEntry::TemplateCalls(sub_calls) => {
                    let mut seq_time = section_start_time;
                    for call in sub_calls {
                        self.compile_template(
                            call,
                            &track.name,
                            track.channel,
                            pitch_offset,
                            scope,
                            &mut seq_time,
                            &mut ctx,
                            1.0,
                        )
                        .with_compile_context(format!(
                            "template call '{}'",
                            call.target.display_name()
                        ))?;
                    }
                    section_max_time = section_max_time.max(seq_time);
                }
                LineEntry::TrackWrap => {
                    section_start_time = section_max_time;
                }
            }
        }
        Ok(section_max_time)
    }

    fn compile_fragment_blocks(
        &self,
        blocks: &[FragmentBlock],
        manifest_tracks: &[Track],
        events: &mut Vec<MidiEvent>,
        pitch_offset: i32,
        swing: Option<(u8, u8)>,
    ) -> CompileResult<()> {
        let mut block_start_time = 0.0;
        let solo_active = manifest_tracks.iter().any(|track| track.solo);

        for block in blocks {
            let mut block_end_time = block_start_time;
            for track in &block.tracks {
                if !track_is_active(track, solo_active) {
                    continue;
                }
                let track_end_time = self
                    .compile_track_sequence(
                        track,
                        events,
                        pitch_offset,
                        TemplateScope {
                            templates: &block.templates,
                            libraries: &block.libraries,
                        },
                        swing,
                        block_start_time,
                    )
                    .with_compile_context(format!(
                        "fragment '{}' track '{}'",
                        block.name, track.name
                    ))?;
                block_end_time = block_end_time.max(track_end_time);
            }
            block_start_time = block_end_time;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn compile_pattern_line(
        &self,
        line: &Line,
        track_name: &str,
        context: &str,
        channel: u8,
        events: &mut Vec<MidiEvent>,
        pitch_offset: i32,
        current_time: &mut f64,
        swing: Option<(u8, u8)>,
        time_scale: f64,
    ) -> CompileResult<()> {
        let channel = crate::validation::to_zero_based_channel(channel).map_err(|_| {
            CompileError::InvalidChannel {
                channel,
                context: format!("{} in track '{}'", context, track_name),
            }
        })?;
        let initial_time = *current_time;
        let mut line_time = initial_time;
        let last_event_indices: Vec<usize> = Vec::new();

        let resolved = self.resolve_modifiers(line, track_name, context)?;

        let mut line_compiler = LineCompiler {
            channel,
            notes: &line.notes,
            events,
            last_event_indices,
            pitch_offset,
            track_name,
            context,
            resolved_velocities: &resolved.velocities,
            resolved_pitches: &resolved.pitches,
            current_block_idx: 0,
            leaf_counter: 0,
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
                            let block_duration = self.unit_per_block * time_scale;
                            line_compiler.current_block_idx = buf_idx;
                            line_compiler.leaf_counter = 0;
                            line_compiler
                                .process_tokens(&buffered_block.tokens, line_time, block_duration)
                                .with_compile_context("repeat end expansion")?;
                            line_time += block_duration;
                        }
                        repeat_buffer.clear();
                        in_repeat = false;
                    }
                }
                Bar::Double => {
                    if in_repeat {
                        for &(buf_idx, buffered_block) in &repeat_buffer {
                            let block_duration = self.unit_per_block * time_scale;
                            line_compiler.current_block_idx = buf_idx;
                            line_compiler.leaf_counter = 0;
                            line_compiler
                                .process_tokens(&buffered_block.tokens, line_time, block_duration)
                                .with_compile_context("double bar repeat expansion")?;
                            line_time += block_duration;
                        }
                        repeat_buffer.clear();
                    }
                    in_repeat = true;
                }
                Bar::Standard => {}
            }

            let block_duration = self.unit_per_block * time_scale;
            line_compiler.current_block_idx = block_idx;
            line_compiler.leaf_counter = 0;
            line_compiler
                .process_tokens(&block.tokens, line_time, block_duration)
                .with_compile_context("pattern block emission")?;
            line_time += block_duration;

            if in_repeat {
                repeat_buffer.push((block_idx, block));
            }
        }

        match line.end_bar {
            Bar::RepeatEnd | Bar::Double => {
                if in_repeat {
                    for &(buf_idx, buffered_block) in &repeat_buffer {
                        let block_duration = self.unit_per_block * time_scale;
                        line_compiler.current_block_idx = buf_idx;
                        line_compiler.leaf_counter = 0;
                        line_compiler
                            .process_tokens(&buffered_block.tokens, line_time, block_duration)
                            .with_compile_context("line end repeat expansion")?;
                        line_time += block_duration;
                    }
                }
            }
            _ => {}
        }

        *current_time = line_time;
        Ok(())
    }
}

pub fn compile_track_init_events(song: &Song) -> CompileResult<Vec<MidiEvent>> {
    let mut out = Vec::new();
    let solo_active = song.tracks.iter().any(|track| track.solo);

    for track in &song.tracks {
        if !track_is_active(track, solo_active) {
            continue;
        }
        let channel = crate::validation::to_zero_based_channel(track.channel).map_err(|_| {
            CompileError::InvalidChannel {
                channel: track.channel,
                context: format!("track init events for '{}'", track.name),
            }
        })?;

        for event in &track.init_events {
            match event {
                TrackInitEvent::BankSelect { msb, lsb } => {
                    out.push(MidiEvent::ControlChange {
                        time: 0.0,
                        channel,
                        cc: 0,
                        value: *msb,
                    });
                    out.push(MidiEvent::ControlChange {
                        time: 0.0,
                        channel,
                        cc: 32,
                        value: *lsb,
                    });
                }
                TrackInitEvent::ProgramChange { program } => {
                    out.push(MidiEvent::ProgramChange {
                        time: 0.0,
                        channel,
                        program: *program,
                    });
                }
                TrackInitEvent::ControlChange { cc, value } => {
                    out.push(MidiEvent::ControlChange {
                        time: 0.0,
                        channel,
                        cc: *cc,
                        value: *value,
                    });
                }
            }
        }
    }

    Ok(out)
}

fn track_is_active(track: &Track, solo_active: bool) -> bool {
    if solo_active {
        track.solo && !track.muted
    } else {
        !track.muted
    }
}

#[cfg(test)]
mod tests {
    use super::{compile_track_init_events, Compiler, MidiEvent};
    use crate::dsl::parser::parse_song;

    fn note_count(events: &[MidiEvent]) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, MidiEvent::Note { .. }))
            .count()
    }

    #[test]
    fn compile_without_solo_keeps_all_unmuted_tracks() {
        let song = parse_song(
            "# Piano: 1\nC4 | ^ |\n\n# Bass: 2\nC2 | ^ |\n\n# Drums: 10 x\nkick | ^ |\n"
                .to_string(),
        )
        .unwrap();
        let events = Compiler::new(&song).unwrap().compile(&song).unwrap();
        assert_eq!(note_count(&events), 2);
    }

    #[test]
    fn compile_with_solo_limits_output_to_solo_tracks() {
        let song = parse_song(
            "# Piano: 1 s\nC4 | ^ |\n\n# Bass: 2\nC2 | ^ |\n\n# Lead: 3 s\nE4 | ^ |\n".to_string(),
        )
        .unwrap();
        let events = Compiler::new(&song).unwrap().compile(&song).unwrap();
        assert_eq!(note_count(&events), 2);
    }

    #[test]
    fn compile_ignores_muted_solo_tracks() {
        let song = parse_song(
            "# Piano: 1 s x\nC4 | ^ |\n\n# Bass: 2 s\nC2 | ^ |\n\n# Lead: 3\nE4 | ^ |\n"
                .to_string(),
        )
        .unwrap();
        let events = Compiler::new(&song).unwrap().compile(&song).unwrap();
        assert_eq!(note_count(&events), 1);
    }

    #[test]
    fn compile_track_init_events_follow_solo_filter() {
        let song = parse_song(
            "# Piano: 1 s\n## pc 1\nC4 | ^ |\n\n# Bass: 2\n## pc 2\nC2 | ^ |\n".to_string(),
        )
        .unwrap();
        let events = compile_track_init_events(&song).unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            MidiEvent::ProgramChange {
                channel: 0,
                program: 1,
                ..
            }
        ));
    }

    #[test]
    fn template_time_scale_applies_to_line_end_repeat_expansion() {
        let song =
            parse_song("# Lead: 1\n[@riff /2]\n\n# @riff\nC4 |: ^ | ^ :|\n".to_string()).unwrap();
        let events = Compiler::new(&song).unwrap().compile(&song).unwrap();
        let note_times: Vec<f64> = events
            .iter()
            .filter_map(|event| match event {
                MidiEvent::Note { time, .. } => Some(*time),
                _ => None,
            })
            .collect();

        assert_eq!(note_times, vec![0.0, 2.0, 4.0, 6.0]);
    }
}
