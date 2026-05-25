use super::{configure_textarea_style, CompileStatus, SourceUndoEntry, StudioApp};
use crate::compiler;
use crate::dsl::{formatter, parser};
use miette::{IntoDiagnostic, Result};
use ratatui_textarea::{CursorMove, TextArea};
use std::fs;

fn resolve_undo_cursor(
    target_cursor: (usize, usize),
    undo_cursor: (usize, usize),
) -> (usize, usize) {
    if target_cursor == undo_cursor {
        target_cursor
    } else {
        undo_cursor
    }
}

impl StudioApp {
    pub(super) fn source(&self) -> String {
        let mut source = self.textarea.lines().join("\n");
        source.push('\n');
        source
    }

    pub(super) fn replace_source(&mut self, source: String) {
        self.textarea = TextArea::from(source.lines());
        configure_textarea_style(&mut self.textarea);
    }

    pub(super) fn push_source_undo(&mut self) {
        let cursor = self.textarea.cursor();
        self.source_undo_stack.push(SourceUndoEntry {
            source: self.source(),
            cursor: (cursor.0, cursor.1),
        });
        const MAX_SOURCE_UNDO: usize = 32;
        if self.source_undo_stack.len() > MAX_SOURCE_UNDO {
            self.source_undo_stack.remove(0);
        }
    }

    pub(super) fn apply_unit_selection_update(
        &mut self,
        lines: Vec<String>,
        selected_indices: &[usize],
        status_message: String,
        audition: Option<(usize, String)>,
    ) -> Result<()> {
        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_unit_selection_from_indices(selected_indices);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = status_message;
        self.audition_candidate(audition);
        Ok(())
    }

    pub(super) fn apply_cursor_source_update(
        &mut self,
        lines: Vec<String>,
        cursor: (usize, usize),
        status_message: String,
        audition: Option<(usize, String)>,
    ) -> Result<()> {
        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.textarea
            .move_cursor(CursorMove::Jump(cursor.0 as u16, cursor.1 as u16));
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = status_message;
        self.audition_candidate(audition);
        Ok(())
    }

    pub(super) fn undo_last_source_edit_to(
        &mut self,
        target_cursor: (usize, usize),
    ) -> Result<bool> {
        let Some(SourceUndoEntry {
            source,
            cursor: undo_cursor,
        }) = self.source_undo_stack.pop()
        else {
            return Ok(false);
        };

        self.replace_source(source);
        self.dirty = true;
        self.compile_and_update_current_source()?;

        let desired_cursor = resolve_undo_cursor(target_cursor, undo_cursor);

        if let Some(token) = self.unit_at_or_after_cursor(desired_cursor.0, desired_cursor.1) {
            self.focus_unit_cursor(&token);
        } else {
            let row = desired_cursor
                .0
                .min(self.textarea.lines().len().saturating_sub(1));
            let col = desired_cursor.1.min(self.line_len(row));
            self.textarea
                .move_cursor(CursorMove::Jump(row as u16, col as u16));
        }

        Ok(true)
    }

    pub(super) fn save(&mut self) -> Result<()> {
        fs::write(&self.path, self.source()).into_diagnostic()?;
        self.dirty = false;
        self.compile_and_update_current_source()?;
        self.status_message = format!("Saved {}", self.path.display());
        Ok(())
    }

    pub(super) fn format_current_source(&mut self) -> Result<()> {
        match formatter::format_string(&self.source()) {
            Ok(formatted) => {
                let cursor = self.textarea.cursor();
                self.push_source_undo();
                self.replace_source(formatted);
                self.textarea
                    .move_cursor(CursorMove::Jump(cursor.0 as u16, cursor.1 as u16));
                self.dirty = true;
                self.compile_and_update_current_source()?;
                self.status_message = "Formatted".into();
            }
            Err(e) => {
                self.status_message = format!("Format failed: {}", e);
            }
        }
        Ok(())
    }

    pub(super) fn compile_and_update_current_source(&mut self) -> Result<()> {
        match parser::parse_song(self.source()) {
            Ok(song) => match compiler::Compiler::new(&song) {
                Ok(compiler_inst) => match compiler_inst.compile(&song) {
                    Ok(events) => {
                        let events: Vec<crate::compiler::MidiEvent> = events.to_vec();
                        let bpm = song.metadata.bpm;
                        let note_count = events
                            .iter()
                            .filter(|e| matches!(e, crate::compiler::MidiEvent::Note { .. }))
                            .count();
                        let control_count = events
                            .iter()
                            .filter(|e| {
                                matches!(
                                    e,
                                    crate::compiler::MidiEvent::ControlChange { .. }
                                        | crate::compiler::MidiEvent::ProgramChange { .. }
                                )
                            })
                            .count();
                        self.bpm = bpm;
                        self.player.update(events, song.metadata);
                        self.compile_status = CompileStatus::Ok {
                            notes: note_count,
                            controls: control_count,
                            bpm,
                        };
                    }
                    Err(e) => {
                        self.compile_status = CompileStatus::Error(format!("Compile error: {}", e));
                    }
                },
                Err(e) => {
                    self.compile_status =
                        CompileStatus::Error(format!("Compiler init error: {}", e));
                }
            },
            Err(e) => {
                self.compile_status = CompileStatus::Error(format!("Parse error: {}", e));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_undo_cursor;

    #[test]
    fn undo_cursor_prefers_snapshot_over_advanced_cursor_for_rest_entry() {
        let rest_token_cursor = (0, 9);
        let advanced_cursor = (0, 11);

        let resolved = resolve_undo_cursor(advanced_cursor, rest_token_cursor);

        assert_eq!(resolved, rest_token_cursor);
    }

    #[test]
    fn undo_cursor_prefers_snapshot_over_advanced_cursor_for_sustain_entry() {
        let sustain_token_cursor = (2, 13);
        let advanced_cursor = (2, 15);

        let resolved = resolve_undo_cursor(advanced_cursor, sustain_token_cursor);

        assert_eq!(resolved, sustain_token_cursor);
    }

    #[test]
    fn undo_cursor_keeps_snapshot_when_cursor_did_not_advance() {
        let current_cursor = (1, 7);

        let resolved = resolve_undo_cursor(current_cursor, current_cursor);

        assert_eq!(resolved, current_cursor);
    }
}
