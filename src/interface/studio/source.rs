use super::{
    configure_textarea_style, CompileStatus, FileNavigationEntry, SourceUndoEntry, StudioApp,
};
use crate::compiler;
use crate::dsl::parser::ParsedLine;
use crate::dsl::token::Frontmatter;
use crate::dsl::{formatter, parser};
use miette::{IntoDiagnostic, Result};
use ratatui_textarea::{CursorMove, TextArea};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

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
        let parsed = self.parse_current_studio_song();
        match parsed {
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
                        if self.manifest_path.is_none() && !song.fragment_blocks.is_empty() {
                            self.manifest_path = Some(self.path.clone());
                        }
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

    fn parse_current_studio_song(&self) -> Result<crate::dsl::token::Song> {
        if let Some(manifest_path) = &self.manifest_path {
            if manifest_path != &self.path {
                let mut overrides = HashMap::new();
                overrides.insert(self.path.clone(), self.source());
                return parser::parse_song_from_path_with_fragment_overrides(
                    manifest_path,
                    &overrides,
                );
            }
        }

        let base_dir = self.path.parent().unwrap_or_else(|| Path::new("."));
        parser::parse_song_with_base_dir(self.source(), base_dir).map_err(Into::into)
    }

    pub(super) fn goto_current_definition(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        if self
            .textarea
            .lines()
            .get(cursor.0)
            .and_then(|line| fragment_call_name(line))
            .is_some()
        {
            return self.goto_current_fragment();
        }
        self.goto_current_template_definition()
    }

    fn goto_current_fragment(&mut self) -> Result<()> {
        if self.dirty {
            self.status_message = "Save before changing file".into();
            return Ok(());
        }

        let cursor = self.textarea.cursor();
        let Some(line) = self.textarea.lines().get(cursor.0) else {
            self.status_message = "No fragment call at cursor".into();
            return Ok(());
        };
        let Some(name) = fragment_call_name(line) else {
            self.status_message = "No fragment call at cursor".into();
            return Ok(());
        };

        let source = self.source();
        let Some(target) = resolve_fragment_target(&source, &self.path, &name)? else {
            self.status_message = format!("Fragment '{}' is not mapped", name);
            return Ok(());
        };
        let manifest_path = self
            .manifest_path
            .clone()
            .unwrap_or_else(|| self.path.clone());
        self.open_file_from_current(target, Some(manifest_path))
    }

    pub(super) fn navigate_back_file(&mut self) -> Result<()> {
        if self.dirty {
            self.status_message = "Save before changing file".into();
            return Ok(());
        }

        let Some(entry) = self.file_navigation_stack.pop() else {
            self.status_message = "No previous file".into();
            return Ok(());
        };

        self.open_file_entry(entry)
    }

    fn open_file_from_current(
        &mut self,
        target: PathBuf,
        manifest_path: Option<PathBuf>,
    ) -> Result<()> {
        let cursor = self.textarea.cursor();
        self.file_navigation_stack.push(FileNavigationEntry {
            path: self.path.clone(),
            cursor: (cursor.0, cursor.1),
            manifest_path: self.manifest_path.clone(),
        });

        self.open_file_entry(FileNavigationEntry {
            path: target,
            cursor: (0, 0),
            manifest_path,
        })
    }

    fn open_file_entry(&mut self, entry: FileNavigationEntry) -> Result<()> {
        let source = fs::read_to_string(&entry.path).into_diagnostic()?;
        self.path = entry.path;
        self.manifest_path = entry.manifest_path;
        self.replace_source(source);
        let row = entry
            .cursor
            .0
            .min(self.textarea.lines().len().saturating_sub(1));
        let col = entry.cursor.1.min(self.line_len(row));
        self.textarea
            .move_cursor(CursorMove::Jump(row as u16, col as u16));
        self.selection = None;
        self.source_undo_stack.clear();
        self.continuous_input_history.clear();
        self.dirty = false;
        self.compile_and_update_current_source()?;
        self.status_message = format!("Opened {}", self.path.display());
        Ok(())
    }
}

fn fragment_call_name(line: &str) -> Option<String> {
    match parser::parse_line_entry(line.trim()).ok()?.1 {
        ParsedLine::FragmentCall { name } => Some(name),
        _ => None,
    }
}

fn resolve_fragment_target(
    source: &str,
    manifest_path: &Path,
    name: &str,
) -> Result<Option<PathBuf>> {
    let Some(frontmatter) = parse_source_frontmatter(source)? else {
        return Ok(None);
    };
    let Some(mapped) = frontmatter.fragments.get(name) else {
        return Ok(None);
    };
    let mapped_path = Path::new(mapped);
    if mapped_path.is_absolute() || mapped_path.components().any(|c| c == Component::ParentDir) {
        return Ok(None);
    }
    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    Ok(Some(base_dir.join(mapped_path)))
}

fn parse_source_frontmatter(source: &str) -> Result<Option<Frontmatter>> {
    let mut lines = source.lines();
    if lines.next() != Some("---") {
        return Ok(None);
    }

    let mut yaml = String::new();
    for line in lines {
        if line == "---" {
            let metadata = serde_yaml::from_str(&yaml).into_diagnostic()?;
            return Ok(Some(metadata));
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::{
        fragment_call_name, parse_source_frontmatter, resolve_fragment_target, resolve_undo_cursor,
    };
    use std::path::Path;

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

    #[test]
    fn fragment_call_name_reads_wikilink_line() {
        assert_eq!(
            fragment_call_name("  [[intro-a]]  "),
            Some("intro-a".to_string())
        );
        assert_eq!(fragment_call_name("# Lead: 1"), None);
    }

    #[test]
    fn resolve_fragment_target_uses_manifest_frontmatter_mapping() {
        let source = r#"---
fragments:
  intro: sections/intro.loom
---

[[intro]]
"#;

        let target = resolve_fragment_target(source, Path::new("song.loom"), "intro")
            .unwrap()
            .unwrap();

        assert_eq!(target, Path::new("sections/intro.loom"));
    }

    #[test]
    fn parse_source_frontmatter_reads_fragments() {
        let source = "---\nfragments:\n  intro: sections/intro.loom\n---\n";

        let frontmatter = parse_source_frontmatter(source).unwrap().unwrap();

        assert_eq!(
            frontmatter.fragments.get("intro").map(String::as_str),
            Some("sections/intro.loom")
        );
    }
}
