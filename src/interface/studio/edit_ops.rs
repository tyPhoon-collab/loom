use super::selection::{
    bar_spans_in_line, char_range, delete_note_token, insert_at_col, is_seq_line,
    note_spans_in_line, replace_char_range, SelectableTokenKind, StudioSelection,
};
use super::transform::{transpose_bar_text, transpose_line, transpose_note_token};
use super::{StudioApp, StudioMode};
use miette::Result;
use ratatui_textarea::CursorMove;

impl StudioApp {
    pub(super) fn transpose_selection(&mut self, semitones: i32) -> Result<()> {
        if matches!(
            self.selection,
            Some(StudioSelection::Note { .. } | StudioSelection::NoteRange { .. })
        ) {
            return self.transpose_selected_notes(semitones);
        }
        if matches!(
            self.selection,
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. })
        ) {
            return self.transpose_selected_bars(semitones);
        }

        let (start, end) = self.selected_line_range();
        let mut lines = self.textarea.lines().to_vec();
        let mut changed = 0usize;

        for row in start..=end {
            if let Some(line) = lines.get_mut(row) {
                let (new_line, line_changed) = transpose_line(line, semitones)?;
                if line_changed {
                    *line = new_line;
                    changed += 1;
                }
            }
        }

        let cursor = self.textarea.cursor();
        let audition = self.audition_candidate_from_lines(&lines, start, end, (cursor.0, cursor.1));

        if changed == 0 {
            self.status_message = "No transposable notes in selection".into();
            return Ok(());
        }

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.textarea
            .move_cursor(CursorMove::Jump(cursor.0 as u16, cursor.1 as u16));
        if self.selection.is_some() {
            self.sync_selection_visual();
        }
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Transposed {} line{} by {:+} semitone{}",
            changed,
            if changed == 1 { "" } else { "s" },
            semitones,
            if semitones.abs() == 1 { "" } else { "s" }
        );
        self.audition_candidate(audition);
        Ok(())
    }

    pub(super) fn transpose_selected_bars(&mut self, semitones: i32) -> Result<()> {
        let selected_bars = self.selected_bar_spans();
        if selected_bars.is_empty() {
            self.status_message = "No bar selected".into();
            return Ok(());
        }

        let mut lines = self.textarea.lines().to_vec();
        let mut replacements = Vec::new();
        for bar in &selected_bars {
            let Some(line) = lines.get(bar.row) else {
                self.status_message = "Selected bar no longer exists".into();
                return Ok(());
            };
            let bar_text = char_range(line, bar.start_col, bar.end_col);
            let (new_bar_text, changed) = transpose_bar_text(&bar_text, semitones)?;
            if changed {
                replacements.push((bar.clone(), new_bar_text));
            }
        }

        if replacements.is_empty() {
            self.status_message = "No transposable notes in selected bars".into();
            return Ok(());
        }

        replacements.sort_by(|(left, _), (right, _)| {
            right
                .row
                .cmp(&left.row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });
        for (bar, new_bar_text) in &replacements {
            let Some(line) = lines.get_mut(bar.row) else {
                self.status_message = "Selected bar no longer exists".into();
                return Ok(());
            };
            replace_char_range(line, bar.start_col, bar.end_col, new_bar_text);
        }

        let selected_rows_indices: Vec<(usize, usize)> = selected_bars
            .iter()
            .map(|bar| (bar.row, bar.index))
            .collect();
        let audition = replacements.iter().find_map(|(old_bar, _)| {
            let line = lines.get(old_bar.row)?;
            let new_bar = bar_spans_in_line(old_bar.row, line)
                .into_iter()
                .find(|bar| bar.index == old_bar.index)?;
            note_spans_in_line(old_bar.row, line)
                .into_iter()
                .find(|note| {
                    note.kind == SelectableTokenKind::Note
                        && note.start_col >= new_bar.start_col
                        && note.end_col <= new_bar.end_col
                })
                .map(|note| (note.row, note.token))
        });

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_bar_selection_from_positions(&selected_rows_indices);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Transposed {} bar{} by {:+}",
            selected_bars.len(),
            if selected_bars.len() == 1 { "" } else { "s" },
            semitones
        );
        self.audition_candidate(audition);
        Ok(())
    }

    pub(super) fn transpose_selected_notes(&mut self, semitones: i32) -> Result<()> {
        let selected_indices = self.selected_note_indices();
        let selected_notes = self.selected_note_spans();
        if selected_notes.is_empty() {
            self.status_message = "No editable token selected".into();
            return Ok(());
        }

        let mut replacements = Vec::new();
        for note in selected_notes {
            if note.kind != SelectableTokenKind::Note {
                continue;
            }
            let mut changed = false;
            let new_token = transpose_note_token(&note.token, semitones, &mut changed)?;
            if changed {
                replacements.push((note, new_token));
            }
        }

        let audition = replacements
            .first()
            .map(|(note, token)| (note.row, token.clone()));

        if replacements.is_empty() {
            self.status_message = "No transposable note selected".into();
            return Ok(());
        }

        let mut lines = self.textarea.lines().to_vec();
        replacements.sort_by(|(left, _), (right, _)| {
            right
                .row
                .cmp(&left.row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });
        for (note, new_token) in &replacements {
            let Some(line) = lines.get_mut(note.row) else {
                self.status_message = "Selected token no longer exists".into();
                return Ok(());
            };
            replace_char_range(line, note.start_col, note.end_col, new_token);
        }

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_note_selection_from_indices(&selected_indices);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Transposed {} note{} by {:+}",
            replacements.len(),
            if replacements.len() == 1 { "" } else { "s" },
            semitones
        );
        self.audition_candidate(audition);
        Ok(())
    }

    pub(super) fn replace_selected_tokens(&mut self, replacement: &str) -> Result<()> {
        let selected_indices = self.selected_note_indices();
        let mut selected_notes = self.selected_note_spans();
        if selected_notes.is_empty() {
            self.status_message = "Replacement applies to editable token selection only".into();
            return Ok(());
        }

        let replacement_name = match replacement {
            "." => "Rested",
            "-" => "Sustained",
            _ => "Replaced",
        };
        selected_notes.sort_by(|left, right| {
            right
                .row
                .cmp(&left.row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });

        let mut lines = self.textarea.lines().to_vec();
        for note in &selected_notes {
            let Some(line) = lines.get_mut(note.row) else {
                self.status_message = "Selected token no longer exists".into();
                return Ok(());
            };
            replace_char_range(line, note.start_col, note.end_col, replacement);
        }

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_note_selection_from_indices(&selected_indices);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "{} {} token{}",
            replacement_name,
            selected_notes.len(),
            if selected_notes.len() == 1 { "" } else { "s" }
        );
        Ok(())
    }

    pub(super) fn delete_selected_notes(&mut self) -> Result<()> {
        let mut selected_notes = self.selected_note_spans();
        if selected_notes.is_empty() {
            self.status_message = "x applies to editable token selection only".into();
            return Ok(());
        }

        let first = selected_notes.first().cloned();
        selected_notes.sort_by(|left, right| {
            right
                .row
                .cmp(&left.row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });

        let mut lines = self.textarea.lines().to_vec();
        for note in &selected_notes {
            let Some(line) = lines.get_mut(note.row) else {
                self.status_message = "Selected token no longer exists".into();
                return Ok(());
            };
            delete_note_token(line, note);
        }

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        if let Some(note) = first {
            self.textarea
                .move_cursor(CursorMove::Jump(note.row as u16, note.start_col as u16));
        }
        self.selection = None;
        self.textarea.cancel_selection();
        self.mode = StudioMode::Normal;
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Deleted {} token{}",
            selected_notes.len(),
            if selected_notes.len() == 1 { "" } else { "s" }
        );
        Ok(())
    }

    pub(super) fn delete_selection(&mut self) -> Result<()> {
        if matches!(
            self.selection,
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. })
        ) {
            self.delete_selected_bars()
        } else {
            self.delete_selected_notes()
        }
    }

    pub(super) fn delete_selected_bars(&mut self) -> Result<()> {
        let selected_bars = self.selected_bar_spans();
        if selected_bars.is_empty() {
            self.status_message = "x applies to bar selection only".into();
            return Ok(());
        }

        let row = selected_bars[0].row;
        if selected_bars.iter().any(|bar| bar.row != row) {
            self.status_message = "Bar delete currently supports one line at a time".into();
            return Ok(());
        }
        if selected_bars.len() == self.bar_spans_on_line(row).len() {
            self.status_message = "Cannot delete all bars on a line".into();
            return Ok(());
        }

        let first = selected_bars.first().cloned().unwrap();
        let last = selected_bars.last().cloned().unwrap();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(row) else {
            self.status_message = "Selected bar no longer exists".into();
            return Ok(());
        };
        replace_char_range(line, first.start_col + 1, last.end_col, "");

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.textarea
            .move_cursor(CursorMove::Jump(row as u16, first.start_col as u16));
        self.selection = None;
        self.textarea.cancel_selection();
        self.mode = StudioMode::Normal;
        self.dirty = true;
        self.compile_and_update_current_source()?;
        let mut status_message = format!(
            "Deleted {} bar{}",
            selected_bars.len(),
            if selected_bars.len() == 1 { "" } else { "s" }
        );
        if self.current_loop_range().is_some() {
            status_message.push_str(" | loop_range unchanged");
        }
        self.status_message = status_message;
        Ok(())
    }

    pub(super) fn duplicate_selection(&mut self) -> Result<()> {
        if matches!(
            self.selection,
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. })
        ) {
            self.duplicate_selected_bars()
        } else {
            self.duplicate_selected_notes()
        }
    }

    pub(super) fn duplicate_selected_notes(&mut self) -> Result<()> {
        let selected_indices = self.selected_note_indices();
        let selected_notes = self.selected_note_spans();
        if selected_notes.is_empty() {
            self.status_message = "d applies to editable token selection only".into();
            return Ok(());
        }

        let row = selected_notes[0].row;
        if selected_notes.iter().any(|note| note.row != row) {
            self.status_message = "Duplicate currently supports one seq line at a time".into();
            return Ok(());
        }

        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(row) else {
            self.status_message = "Selected token no longer exists".into();
            return Ok(());
        };
        if !is_seq_line(line) {
            self.status_message = "Duplicate currently supports seq body tokens only".into();
            return Ok(());
        }

        let Some(last_note) = selected_notes.last() else {
            return Ok(());
        };
        let insertion = format!(
            " {}",
            selected_notes
                .iter()
                .map(|note| note.token.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        );
        insert_at_col(line, last_note.end_col, &insertion);

        let Some(last_selected_index) = selected_indices.iter().max().copied() else {
            self.status_message = "Selected token no longer exists".into();
            return Ok(());
        };
        let inserted_indices: Vec<usize> =
            (last_selected_index + 1..=last_selected_index + selected_notes.len()).collect();

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_note_selection_from_indices(&inserted_indices);
        self.sync_selection_visual();
        let audition = self.audition_candidate_from_indices(&inserted_indices);
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Duplicated {} token{}",
            selected_notes.len(),
            if selected_notes.len() == 1 { "" } else { "s" }
        );
        self.audition_candidate(audition);
        Ok(())
    }

    pub(super) fn duplicate_selected_bars(&mut self) -> Result<()> {
        let selected_bars = self.selected_bar_spans();
        if selected_bars.is_empty() {
            self.status_message = "d applies to bar selection only".into();
            return Ok(());
        }

        let row = selected_bars[0].row;
        if selected_bars.iter().any(|bar| bar.row != row) {
            self.status_message = "Bar duplicate currently supports one line at a time".into();
            return Ok(());
        }

        let first = selected_bars.first().cloned().unwrap();
        let last = selected_bars.last().cloned().unwrap();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(row) else {
            self.status_message = "Selected bar no longer exists".into();
            return Ok(());
        };

        let insertion = char_range(line, first.start_col + 1, last.end_col);
        insert_at_col(line, last.end_col, &insertion);

        let inserted_indices: Vec<usize> =
            (last.index + 1..=last.index + selected_bars.len()).collect();

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_bar_selection_from_row_indices(row, &inserted_indices);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Duplicated {} bar{}",
            selected_bars.len(),
            if selected_bars.len() == 1 { "" } else { "s" }
        );
        Ok(())
    }
}
