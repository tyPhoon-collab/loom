use super::macros::apply_macro;
use super::{CompileContextExt, CompileError, CompileResult, Compiler, MidiEvent};
use crate::dsl::token::{
    LineEntry, TemplateCall, TemplateCallTarget, TemplateDef, TemplateLibrary, TemplateMacro,
    TemplateParam,
};

type TemplateMap = std::collections::HashMap<String, TemplateDef>;
type LibraryMap = std::collections::HashMap<String, TemplateLibrary>;

pub(super) struct CompilerContext<'a> {
    pub(super) events: &'a mut Vec<MidiEvent>,
    pub(super) call_stack: &'a mut Vec<String>,
    pub(super) swing: Option<(u8, u8)>,
}

#[derive(Clone, Copy)]
pub(super) struct TemplateScope<'a> {
    pub(super) templates: &'a TemplateMap,
    pub(super) libraries: &'a LibraryMap,
}

fn resolve_template_call<'a>(
    call: &TemplateCall,
    scope: TemplateScope<'a>,
    track_name: &str,
    ctx: &CompilerContext,
) -> CompileResult<(&'a TemplateDef, TemplateScope<'a>, String)> {
    match &call.target {
        TemplateCallTarget::Local { name } => {
            let display_name = format!("@{}", name);
            let def = scope
                .templates
                .get(name)
                .ok_or_else(|| CompileError::TemplateNotFound {
                    template: name.clone(),
                    context: format!(
                        "track '{}', stack [{}]",
                        track_name,
                        call_stack_with(ctx, &display_name)
                    ),
                })?;
            Ok((def, scope, display_name))
        }
        TemplateCallTarget::Library { alias, name } => {
            let library =
                scope
                    .libraries
                    .get(alias)
                    .ok_or_else(|| CompileError::TemplateNotFound {
                        template: format!("{}.{}", alias, name),
                        context: format!(
                            "track '{}', template library alias '{}' not found, stack [{}]",
                            track_name,
                            alias,
                            call_stack_with(ctx, &format!("{}.{}", alias, name))
                        ),
                    })?;
            let display_name = format!("{}:@{}", library.source, name);
            let def =
                library
                    .templates
                    .get(name)
                    .ok_or_else(|| CompileError::TemplateNotFound {
                        template: name.clone(),
                        context: format!(
                            "track '{}', template library '{}' ({})",
                            track_name, alias, library.source
                        ),
                    })?;
            Ok((
                def,
                TemplateScope {
                    templates: &library.templates,
                    libraries: &library.libraries,
                },
                display_name,
            ))
        }
    }
}

fn call_stack_with(ctx: &CompilerContext, name: &str) -> String {
    ctx.call_stack
        .iter()
        .chain(std::iter::once(&name.to_string()))
        .cloned()
        .collect::<Vec<_>>()
        .join(" -> ")
}

impl Compiler {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn compile_template(
        &self,
        call: &TemplateCall,
        track_name: &str,
        channel: u8,
        mut pitch_offset: i32,
        scope: TemplateScope,
        current_time: &mut f64,
        ctx: &mut CompilerContext,
        parent_time_scale: f64,
    ) -> CompileResult<()> {
        let (def, nested_scope, display_name) =
            resolve_template_call(call, scope, track_name, ctx)?;

        if ctx.call_stack.contains(&display_name) {
            let trace = ctx.call_stack.join(" -> ") + " -> " + &display_name;
            return Err(CompileError::CircularTemplateReference(trace));
        }
        ctx.call_stack.push(display_name);

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
                                nested_scope,
                                &mut seq_time,
                                ctx,
                                effective_time_scale,
                            )
                            .with_compile_context(format!(
                                "nested template call '{}'",
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
            *current_time = section_max_time;
        }

        let generated_events = &mut ctx.events[events_start_idx..];
        for macro_kind in &note_macros {
            apply_macro(generated_events, macro_kind, effective_time_scale);
        }

        ctx.call_stack.pop();
        Ok(())
    }
}
