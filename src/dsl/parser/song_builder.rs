use crate::dsl::error::ParseError;
use crate::dsl::token::{
    Bar, Block, Line, LineEntry, ModifierBlock, ModifierKind, ModifierLine, Note, TemplateDef,
    Track, TrackInitEvent,
};
use miette::Result;

pub struct SongBuilder<'a> {
    tracks: Vec<Track>,
    templates: std::collections::HashMap<String, TemplateDef>,
    current_track: Option<Track>,
    current_template: Option<(String, crate::dsl::token::Sequence)>,
    source: &'a str,
}

impl<'a> SongBuilder<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            tracks: Vec::new(),
            templates: std::collections::HashMap::new(),
            current_track: None,
            current_template: None,
            source,
        }
    }

    pub fn start_template(&mut self, name: String) {
        self.finish_current();
        self.current_template = Some((
            name,
            crate::dsl::token::Sequence {
                entries: Vec::new(),
            },
        ));
    }

    pub fn finish_current(&mut self) {
        if let Some(t) = self.current_track.take() {
            self.tracks.push(t);
        }
        if let Some((name, sequence)) = self.current_template.take() {
            self.templates
                .insert(name.clone(), TemplateDef { name, sequence });
        }
    }

    pub fn add_track(
        &mut self,
        name: String,
        channel: u8,
        line_str: &str,
        solo: bool,
        muted: bool,
    ) -> Result<(), ParseError> {
        if let Err(msg) = crate::validation::ensure_channel_1_based(channel) {
            return Err(ParseError::from_validation(
                line_str,
                self.source,
                msg,
                Some("MIDI channel must be between 1 and 16. Example: # Piano: 1".to_string()),
            ));
        }

        self.finish_current();
        self.current_track = Some(Track {
            name,
            channel,
            solo,
            muted,
            init_events: Vec::new(),
            sequence: crate::dsl::token::Sequence {
                entries: Vec::new(),
            },
        });
        Ok(())
    }

    pub fn add_section(&mut self, _line_str: &str) -> Result<(), ParseError> {
        if let Some(ref mut track) = self.current_track {
            track.sequence.entries.push(LineEntry::TrackWrap);
        }
        Ok(())
    }

    pub fn add_pattern(
        &mut self,
        _line_str: &str,
        notes: Vec<Note>,
        blocks: Vec<Block>,
        end_bar: Bar,
    ) -> Result<(), ParseError> {
        let entry = LineEntry::Pattern(Line {
            notes,
            blocks,
            end_bar,
            modifiers: Vec::new(),
        });

        if let Some((_, ref mut seq)) = self.current_template {
            seq.entries.push(entry);
        } else if let Some(ref mut track) = self.current_track {
            track.sequence.entries.push(entry);
        }

        Ok(())
    }

    pub fn add_template_calls(&mut self, calls: Vec<crate::dsl::token::TemplateCall>) {
        let entry = LineEntry::TemplateCalls(calls);

        if let Some((_, ref mut seq)) = self.current_template {
            seq.entries.push(entry);
        } else if let Some(ref mut track) = self.current_track {
            track.sequence.entries.push(entry);
        }
    }

    pub fn add_track_init(
        &mut self,
        line_str: &str,
        event: TrackInitEvent,
    ) -> Result<(), ParseError> {
        if self.current_template.is_some() {
            return Err(ParseError::from_context(
                line_str,
                self.source,
                "Track init line (## ...) is not allowed inside template".to_string(),
            ));
        }

        let track = self.current_track.as_mut().ok_or_else(|| {
            ParseError::from_context(
                line_str,
                self.source,
                "Track header required before init line".to_string(),
            )
        })?;

        match &event {
            TrackInitEvent::ProgramChange { .. } => {
                if track
                    .init_events
                    .iter()
                    .any(|e| matches!(e, TrackInitEvent::ProgramChange { .. }))
                {
                    return Err(ParseError::from_validation(
                        line_str,
                        self.source,
                        "Duplicate program change in the same track".to_string(),
                        Some("Use only one `## pc ...` per track.".to_string()),
                    ));
                }
            }
            TrackInitEvent::BankSelect { .. } => {
                if track
                    .init_events
                    .iter()
                    .any(|e| matches!(e, TrackInitEvent::BankSelect { .. }))
                {
                    return Err(ParseError::from_validation(
                        line_str,
                        self.source,
                        "Duplicate bank select in the same track".to_string(),
                        Some("Use only one `## bank ...` per track.".to_string()),
                    ));
                }
                if track.init_events.iter().any(|e| {
                    matches!(e, TrackInitEvent::ControlChange { cc, .. } if *cc == 0 || *cc == 32)
                }) {
                    return Err(ParseError::from_validation(
                        line_str,
                        self.source,
                        "Cannot mix `## bank ...` with `## cc 0 ...` / `## cc 32 ...`".to_string(),
                        None,
                    ));
                }
            }
            TrackInitEvent::ControlChange { cc, .. } => {
                if (*cc == 0 || *cc == 32)
                    && track
                        .init_events
                        .iter()
                        .any(|e| matches!(e, TrackInitEvent::BankSelect { .. }))
                {
                    return Err(ParseError::from_validation(
                        line_str,
                        self.source,
                        "Cannot mix `## cc 0/32 ...` with `## bank ...`".to_string(),
                        None,
                    ));
                }
                if track.init_events.iter().any(
                    |e| matches!(e, TrackInitEvent::ControlChange { cc: prev, .. } if prev == cc),
                ) {
                    return Err(ParseError::from_validation(
                        line_str,
                        self.source,
                        format!("Duplicate CC{} init event in the same track", cc),
                        None,
                    ));
                }
            }
        }

        track.init_events.push(event);
        Ok(())
    }

    pub fn add_modifier(
        &mut self,
        line_str: &str,
        kind: ModifierKind,
        blocks: Vec<ModifierBlock>,
        end_bar: Bar,
        trailing_comment: Option<String>,
    ) -> Result<(), ParseError> {
        let entries = if let Some((_, ref mut seq)) = self.current_template {
            &mut seq.entries
        } else if let Some(ref mut track) = self.current_track {
            &mut track.sequence.entries
        } else {
            return Err(ParseError::from_context(
                line_str,
                self.source,
                "Track or template header required before modifier line".to_string(),
            ));
        };

        let last_entry = entries.last_mut().ok_or_else(|| {
            ParseError::from_context(
                line_str,
                self.source,
                "Pattern line required before modifier line".to_string(),
            )
        })?;

        if let LineEntry::Pattern(ref mut line) = last_entry {
            line.modifiers.push(ModifierLine {
                kind,
                blocks,
                end_bar,
                trailing_comment,
            });
            Ok(())
        } else {
            Err(ParseError::from_context(
                line_str,
                self.source,
                "Modifier cannot follow a template expansion directly".to_string(),
            ))
        }
    }

    pub fn finish(mut self) -> (Vec<Track>, std::collections::HashMap<String, TemplateDef>) {
        self.finish_current();
        (self.tracks, self.templates)
    }
}
