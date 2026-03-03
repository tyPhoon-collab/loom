use crate::dsl::token::{
    Bar, Block, Line, LineEntry, ModifierKind, ModifierValue, Note, Song, TemplateMacro,
    TemplateParam, Token, Track, TrackInitEvent,
};
use miette::{Diagnostic, Result};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum CompileError {
    #[error("Invalid signature '{signature}': {reason}")]
    #[diagnostic(code(loom::compiler::invalid_signature))]
    InvalidSignature { signature: String, reason: String },

    #[error("Invalid MIDI channel {channel} in {context} (expected 1..16)")]
    #[diagnostic(code(loom::compiler::invalid_channel))]
    InvalidChannel { channel: u8, context: String },

    #[error("Template not found: '{template}' while compiling {context}")]
    #[diagnostic(code(loom::compiler::template_not_found))]
    TemplateNotFound { template: String, context: String },

    #[error("Circular template reference detected: {0}")]
    #[diagnostic(code(loom::compiler::circular_template_reference))]
    CircularTemplateReference(String),

    #[error(
        "Velocity out of range: {value} (expected 0..127) at track '{track}', {context}, block {block_index}, leaf {leaf_index}"
    )]
    #[diagnostic(code(loom::compiler::velocity_out_of_range))]
    VelocityOutOfRange {
        track: String,
        context: String,
        block_index: usize,
        leaf_index: usize,
        value: i32,
    },

    #[error(
        "MIDI note out of range: {value} (expected 0..127) for '{note}' at track '{track}', {context}, block {block_index}, leaf {leaf_index}"
    )]
    #[diagnostic(code(loom::compiler::note_out_of_range))]
    NoteOutOfRange {
        track: String,
        context: String,
        block_index: usize,
        leaf_index: usize,
        note: String,
        value: i32,
    },

    #[error(
        "Invalid note '{note}' at track '{track}', {context}, block {block_index}, leaf {leaf_index}: {reason}"
    )]
    #[diagnostic(code(loom::compiler::invalid_note))]
    InvalidNote {
        track: String,
        context: String,
        block_index: usize,
        leaf_index: usize,
        note: String,
        reason: String,
    },

    #[error("while compiling {context}")]
    #[diagnostic(code(loom::compiler::context))]
    Context {
        context: String,
        #[source]
        source: Box<CompileError>,
    },
}

type CompileResult<T> = std::result::Result<T, CompileError>;

trait CompileContextExt<T> {
    fn with_compile_context(self, context: impl Into<String>) -> CompileResult<T>;
}

impl<T> CompileContextExt<T> for CompileResult<T> {
    fn with_compile_context(self, context: impl Into<String>) -> CompileResult<T> {
        self.map_err(|source| CompileError::Context {
            context: context.into(),
            source: Box::new(source),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum MidiEvent {
    Note {
        time: f64,     // Absolute time in beats
        duration: f64, // Duration in beats
        channel: u8,   // 0-based
        note: u8,
        velocity: u8,
    },
    ControlChange {
        time: f64,
        channel: u8, // 0-based
        cc: u8,
        value: u8,
    },
    ProgramChange {
        time: f64,
        channel: u8, // 0-based
        program: u8,
    },
}

impl MidiEvent {
    pub fn time(&self) -> f64 {
        match self {
            Self::Note { time, .. }
            | Self::ControlChange { time, .. }
            | Self::ProgramChange { time, .. } => *time,
        }
    }

    pub fn channel(&self) -> u8 {
        match self {
            Self::Note { channel, .. }
            | Self::ControlChange { channel, .. }
            | Self::ProgramChange { channel, .. } => *channel,
        }
    }

    pub fn note_end_time(&self) -> Option<f64> {
        match self {
            Self::Note { time, duration, .. } => Some(*time + *duration),
            _ => None,
        }
    }

    pub fn timing_order(&self) -> u8 {
        match self {
            Self::ControlChange { cc: 0, .. } => 0,
            Self::ControlChange { cc: 32, .. } => 1,
            Self::ProgramChange { .. } => 2,
            Self::ControlChange { .. } => 3,
            Self::Note { .. } => 10,
        }
    }

    pub fn note(&self) -> Option<u8> {
        match self {
            Self::Note { note, .. } => Some(*note),
            _ => None,
        }
    }

    pub fn velocity(&self) -> Option<u8> {
        match self {
            Self::Note { velocity, .. } => Some(*velocity),
            _ => None,
        }
    }

    pub fn cc(&self) -> Option<u8> {
        match self {
            Self::ControlChange { cc, .. } => Some(*cc),
            _ => None,
        }
    }

    pub fn value(&self) -> Option<u8> {
        match self {
            Self::ControlChange { value, .. } => Some(*value),
            _ => None,
        }
    }

    pub fn program(&self) -> Option<u8> {
        match self {
            Self::ProgramChange { program, .. } => Some(*program),
            _ => None,
        }
    }
}

pub struct Compiler {
    pub unit_per_block: f64, // e.g. 4.0 for 4/4 bar
}

/// Resolved modifier values for a line, flattened per block (DFS leaf order)
struct ResolvedModifiers {
    velocities: Vec<Vec<i32>>, // per block, per leaf token (DFS order)
    pitches: Vec<Vec<i32>>,    // per block, per leaf token (DFS order)
}

/// Count the number of leaf tokens (non-Group) in a token tree via DFS
fn count_leaf_tokens(tokens: &[Token]) -> usize {
    tokens
        .iter()
        .map(|t| match t {
            Token::Group(sub) => count_leaf_tokens(sub),
            _ => 1,
        })
        .sum()
}

/// Expand modifier values to align with pattern leaf tokens.
///
/// - `ModifierValue::Group(vals)` aligns its sub-values with the sub-tokens of the
///   corresponding `Token::Group`.
/// - A scalar modifier value at a Group token position is broadcast to all
///   leaf tokens of that group.
/// - `ModifierValue::Empty` at a Group position fills all leaves with Empty.
fn expand_modifier_values(tokens: &[Token], mod_values: &[ModifierValue]) -> Vec<ModifierValue> {
    let mut result = Vec::new();

    for (i, token) in tokens.iter().enumerate() {
        let mod_val = mod_values.get(i);
        match token {
            Token::Group(sub_tokens) => {
                let leaf_count = count_leaf_tokens(sub_tokens);
                match mod_val {
                    Some(ModifierValue::Group(sub_vals)) => {
                        // Recurse: align sub-values with sub-tokens
                        result.extend(expand_modifier_values(sub_tokens, sub_vals));
                    }
                    Some(val @ (ModifierValue::Set(_) | ModifierValue::Latch(_))) => {
                        // Broadcast scalar to all leaves in the group
                        for _ in 0..leaf_count {
                            result.push(val.clone());
                        }
                    }
                    Some(ModifierValue::Empty) | None => {
                        // Fill with Empty for all leaves
                        for _ in 0..leaf_count {
                            result.push(ModifierValue::Empty);
                        }
                    }
                }
            }
            _ => {
                // Leaf token: use modifier value directly, or Empty
                match mod_val {
                    Some(ModifierValue::Group(_)) => {
                        // Group modifier on a non-group token: ignore, treat as empty
                        result.push(ModifierValue::Empty);
                    }
                    Some(val) => result.push(val.clone()),
                    None => result.push(ModifierValue::Empty),
                }
            }
        }
    }

    result
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
            )
            .with_compile_context(format!("track '{}'", track.name))?;
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
    fn resolve_modifiers(&self, line: &crate::dsl::token::Line) -> ResolvedModifiers {
        let num_blocks = line.blocks.len();

        // Count leaf tokens per block (DFS)
        let leaves_per_block: Vec<usize> = line
            .blocks
            .iter()
            .map(|b| count_leaf_tokens(&b.tokens))
            .collect();

        // Initialize with defaults
        let mut velocities: Vec<Vec<i32>> = leaves_per_block
            .iter()
            .map(|&n| vec![ModifierKind::Velocity.default_value(); n])
            .collect();
        let mut pitches: Vec<Vec<i32>> = leaves_per_block
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
                    expand_modifier_values(&line.blocks[block_idx].tokens, raw_values)
                } else {
                    vec![ModifierValue::Empty; num_leaves]
                };

                for (leaf_idx, target_slot) in
                    target[block_idx].iter_mut().enumerate().take(num_leaves)
                {
                    let mod_val = expanded.get(leaf_idx).unwrap_or(&ModifierValue::Empty);
                    match mod_val {
                        ModifierValue::Set(v) => {
                            *target_slot = *v;
                            latch_value = None; // One-shot: clear latch
                        }
                        ModifierValue::Latch(v) => {
                            *target_slot = *v;
                            latch_value = Some(*v);
                        }
                        ModifierValue::Empty | ModifierValue::Group(_) => {
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
    ) -> CompileResult<()> {
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
                            &mut seq_time,
                            &mut ctx,
                            1.0,
                        )
                        .with_compile_context(format!("template call '{}'", call.name))?;
                    }
                    section_max_time = section_max_time.max(seq_time);
                }
                LineEntry::TrackWrap => {
                    section_start_time = section_max_time;
                }
            }
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
        let last_event_indices: Vec<Option<usize>> = vec![None; line.notes.len()];

        let resolved = self.resolve_modifiers(line);

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
                        let block_duration = self.unit_per_block;
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

    #[allow(clippy::too_many_arguments)]
    fn compile_template(
        &self,
        call: &crate::dsl::token::TemplateCall,
        track_name: &str,
        channel: u8,
        mut pitch_offset: i32,
        current_time: &mut f64,
        ctx: &mut CompilerContext,
        parent_time_scale: f64,
    ) -> CompileResult<()> {
        if ctx.call_stack.contains(&call.name) {
            let trace = ctx.call_stack.join(" -> ") + " -> " + &call.name;
            return Err(CompileError::CircularTemplateReference(trace));
        }
        ctx.call_stack.push(call.name.clone());
        let def = ctx
            .templates
            .get(&call.name)
            .ok_or_else(|| CompileError::TemplateNotFound {
                template: call.name.clone(),
                context: format!(
                    "track '{}', stack [{}]",
                    track_name,
                    ctx.call_stack.join(" -> ")
                ),
            })?;

        let mut template_pitch_offset = 0;
        let mut structural_repeat = 1u32;
        let mut reverse = false;
        let mut time_scale = 1.0f64;
        let mut note_macros: Vec<TemplateMacro> = Vec::new();
        let mut pan: Option<u8> = None;

        for param in &call.params {
            match param {
                TemplateParam::Transpose(v) => template_pitch_offset += v,
                TemplateParam::StructuralRepeat(v) => structural_repeat = *v,
                TemplateParam::TimeScale(v) => time_scale = 1.0 / *v as f64,
                TemplateParam::Macro(TemplateMacro::Rev) => reverse = true,
                TemplateParam::Macro(TemplateMacro::Pan(v)) => pan = Some(*v),
                TemplateParam::Macro(m) => note_macros.push(m.clone()),
            }
        }

        pitch_offset += template_pitch_offset;
        let effective_time_scale = parent_time_scale * time_scale;

        // Track where new events start so we can apply macros later
        let events_start_idx = ctx.events.len();
        if let Some(value) = pan {
            let zero_based_channel =
                crate::validation::to_zero_based_channel(channel).map_err(|_| {
                    CompileError::InvalidChannel {
                        channel,
                        context: format!("template pan macro in track '{}'", track_name),
                    }
                })?;
            ctx.events.push(MidiEvent::ControlChange {
                time: *current_time,
                channel: zero_based_channel,
                cc: 10,
                value,
            });
        }

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
                        let template_ctx =
                            format!("template stack [{}]", ctx.call_stack.join(" -> "));
                        self.compile_pattern_line(
                            &line_repeated,
                            track_name,
                            &template_ctx,
                            channel,
                            ctx.events,
                            pitch_offset,
                            &mut line_time,
                            ctx.swing,
                            effective_time_scale,
                        )
                        .with_compile_context("pattern line in template")?;
                        if line_time > section_max_time {
                            section_max_time = line_time;
                        }
                    }
                    LineEntry::TemplateCalls(sub_calls) => {
                        let mut seq_time = section_start_time;
                        for call in sub_calls {
                            self.compile_template(
                                call,
                                track_name,
                                channel,
                                pitch_offset,
                                &mut seq_time,
                                ctx,
                                effective_time_scale,
                            )
                            .with_compile_context(format!(
                                "nested template call '{}'",
                                call.name
                            ))?;
                        }
                        section_max_time = section_max_time.max(seq_time);
                    }
                    LineEntry::TrackWrap => {
                        section_start_time = section_max_time;
                    }
                }
            }
            *current_time = section_max_time;
        }

        // Apply post-processing macros to generated events
        let generated_events = &mut ctx.events[events_start_idx..];
        for macro_kind in &note_macros {
            apply_macro(generated_events, macro_kind, effective_time_scale);
        }

        ctx.call_stack.pop();
        Ok(())
    }
}

pub fn compile_track_init_events(song: &Song) -> CompileResult<Vec<MidiEvent>> {
    let mut out = Vec::new();

    for track in &song.tracks {
        if track.muted {
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

/// Apply a post-processing macro to a slice of generated MidiEvents.
fn apply_macro(events: &mut [MidiEvent], macro_kind: &TemplateMacro, time_scale: f64) {
    match macro_kind {
        TemplateMacro::Vel(vel) => {
            for event in events.iter_mut() {
                if let MidiEvent::Note { velocity, .. } = event {
                    *velocity = *vel;
                }
            }
        }
        TemplateMacro::Arp => {
            // arp — Spread simultaneous notes evenly across their block duration
            apply_arp(events, time_scale);
        }
        TemplateMacro::Strum => {
            // strum — Small timing offsets between simultaneous notes (guitar-like)
            apply_strum(events, time_scale);
        }
        TemplateMacro::Rev | TemplateMacro::Pan(_) => {}
    }
}

/// Arpeggiate: spread simultaneous notes evenly across the block duration.
fn apply_arp(events: &mut [MidiEvent], _time_scale: f64) {
    if events.is_empty() {
        return;
    }

    // Group events by their start time
    let mut time_groups: std::collections::BTreeMap<u64, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (i, event) in events.iter().enumerate() {
        if let MidiEvent::Note { time, .. } = event {
            let key = (*time * 1_000_000.0) as u64; // Use microsecond precision for grouping
            time_groups.entry(key).or_default().push(i);
        }
    }

    for indices in time_groups.values() {
        let count = indices.len();
        if count <= 1 {
            continue;
        }
        // Get the original duration of the block these notes belong to
        let original_duration = match events[indices[0]] {
            MidiEvent::Note { duration, .. } => duration,
            _ => continue,
        };
        let step = original_duration / count as f64;

        // Sort by pitch (low to high) for natural arpeggio
        let mut sorted_indices: Vec<usize> = indices.clone();
        sorted_indices.sort_by(|&a, &b| {
            let note_a = match events[a] {
                MidiEvent::Note { note, .. } => note,
                _ => 0,
            };
            let note_b = match events[b] {
                MidiEvent::Note { note, .. } => note,
                _ => 0,
            };
            note_a.cmp(&note_b)
        });

        for (nth, &idx) in sorted_indices.iter().enumerate() {
            if let MidiEvent::Note { time, duration, .. } = &mut events[idx] {
                *time += step * nth as f64;
                *duration = step;
            }
        }
    }
}

/// Strum: add small timing offsets between simultaneous notes (guitar-like feel).
fn apply_strum(events: &mut [MidiEvent], _time_scale: f64) {
    if events.is_empty() {
        return;
    }

    let strum_interval = 0.03; // ~30ms worth of beats at moderate tempo

    // Group events by their start time
    let mut time_groups: std::collections::BTreeMap<u64, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (i, event) in events.iter().enumerate() {
        if let MidiEvent::Note { time, .. } = event {
            let key = (*time * 1_000_000.0) as u64;
            time_groups.entry(key).or_default().push(i);
        }
    }

    for indices in time_groups.values() {
        let count = indices.len();
        if count <= 1 {
            continue;
        }

        // Sort by pitch: low notes first (strum from bass string up)
        let mut sorted_indices: Vec<usize> = indices.clone();
        sorted_indices.sort_by(|&a, &b| {
            let note_a = match events[a] {
                MidiEvent::Note { note, .. } => note,
                _ => 0,
            };
            let note_b = match events[b] {
                MidiEvent::Note { note, .. } => note,
                _ => 0,
            };
            note_a.cmp(&note_b)
        });

        for (nth, &idx) in sorted_indices.iter().enumerate() {
            let offset = strum_interval * nth as f64;
            if let MidiEvent::Note { time, duration, .. } = &mut events[idx] {
                *time += offset;
                // Shorten duration slightly to keep the end time the same
                *duration = (*duration - offset).max(0.01);
            }
        }
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
    track_name: &'a str,
    context: &'a str,
    current_block_idx: usize,
    leaf_counter: usize,
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
    fn process_tokens(
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
                    // Get velocity and pitch modifier for this leaf token
                    let leaf_idx = self.leaf_counter;
                    let velocity = self
                        .resolved_velocities
                        .get(self.current_block_idx)
                        .and_then(|v| v.get(leaf_idx))
                        .copied()
                        .unwrap_or(100);
                    let velocity =
                        crate::validation::ensure_u7_i32(velocity, "Velocity").map_err(|_| {
                            CompileError::VelocityOutOfRange {
                                track: self.track_name.to_string(),
                                context: self.context.to_string(),
                                block_index: self.current_block_idx,
                                leaf_index: leaf_idx,
                                value: velocity,
                            }
                        })?;

                    let pitch_mod = self
                        .resolved_pitches
                        .get(self.current_block_idx)
                        .and_then(|v| v.get(leaf_idx))
                        .copied()
                        .unwrap_or(0);

                    for (nth, note) in self.notes.iter().enumerate() {
                        let base_note =
                            note.to_midi_checked()
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
                                crate::validation::ensure_u7_i32(base_note + pitch_mod, "MIDI note")
                                    .map_err(|_| CompileError::NoteOutOfRange {
                                        track: self.track_name.to_string(),
                                        context: self.context.to_string(),
                                        block_index: self.current_block_idx,
                                        leaf_index: leaf_idx,
                                        note: note.to_string(),
                                        value: base_note + pitch_mod,
                                    })?
                            }
                        };
                        let event = MidiEvent::Note {
                            time: token_time,
                            duration: swung_duration,
                            channel: self.channel,
                            note: midi_val,
                            velocity,
                        };
                        self.events.push(event);
                        self.last_event_indices[nth] = Some(self.events.len() - 1);
                    }
                    self.leaf_counter += 1;
                }
                Token::Rest => {
                    for (nth, _) in self.notes.iter().enumerate() {
                        self.last_event_indices[nth] = None;
                    }
                    self.leaf_counter += 1;
                }
                Token::Sustain => {
                    for (nth, _) in self.notes.iter().enumerate() {
                        if let Some(idx) = self.last_event_indices[nth] {
                            if let MidiEvent::Note { duration, .. } = &mut self.events[idx] {
                                *duration += swung_duration;
                            }
                        }
                    }
                    self.leaf_counter += 1;
                }
                Token::Group(sub_tokens) => {
                    // Recurse into group — leaf_counter continues from current position
                    self.process_tokens(sub_tokens, unswung_start, duration_per_token)?;
                }
            }
        }
        Ok(())
    }
}
