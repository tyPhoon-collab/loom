use crate::interface::studio::selection::{
    bar_spans_in_line, char_range, replace_char_range, unit_spans_in_line, StudioSelection,
};
use crate::interface::studio::transform::{
    transpose_bar_text, transpose_line, transpose_note_token,
};
use crate::interface::studio::StudioApp;
use miette::Result;
use ratatui_textarea::CursorMove;

impl StudioApp {
    pub(crate) fn transpose_selection(&mut self, semitones: i32) -> Result<()> {
        if matches!(
            self.selection,
            Some(StudioSelection::Unit { .. } | StudioSelection::UnitRange { .. })
        ) {
            return self.transpose_selected_units(semitones);
        }
        if matches!(
            self.selection,
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. })
        ) {
            return self.transpose_selected_bars(semitones);
        }
        if matches!(
            self.selection,
            Some(StudioSelection::TemplateCall { .. } | StudioSelection::TemplateCallRange { .. })
        ) {
            return self.transpose_selected_template_calls(semitones);
        }
        if self.selection.is_none() {
            if self.current_template_call_at_cursor().is_some() {
                return self.transpose_current_template_call(semitones);
            }
            return self.transpose_current_unit(semitones);
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

    pub(crate) fn transpose_current_unit(&mut self, semitones: i32) -> Result<()> {
        let cursor = self.textarea.cursor();
        let Some(unit) = self.unit_at_or_after_cursor(cursor.0, cursor.1) else {
            self.status_message = "No unit on this line".into();
            return Ok(());
        };
        if !unit.kind.is_pitch() {
            self.status_message = "Transpose applies to pitch units only".into();
            return Ok(());
        }

        let mut changed = false;
        let new_token = transpose_note_token(&unit.token, semitones, &mut changed)?;
        if !changed {
            self.status_message = "No transposable note on current unit".into();
            return Ok(());
        }

        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(unit.row) else {
            self.status_message = "Selected unit no longer exists".into();
            return Ok(());
        };
        replace_char_range(line, unit.start_col, unit.end_col, &new_token);

        self.apply_cursor_source_update(
            lines,
            (unit.row, unit.start_col),
            format!("Transposed current unit by {:+}", semitones),
            Some((unit.row, new_token)),
        )
    }

    pub(crate) fn transpose_selected_bars(&mut self, semitones: i32) -> Result<()> {
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
            unit_spans_in_line(old_bar.row, line)
                .into_iter()
                .find(|note| {
                    note.kind.is_pitch()
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

    pub(crate) fn transpose_selected_units(&mut self, semitones: i32) -> Result<()> {
        let selected_indices = self.selected_unit_indices();
        let selected_tokens = self.selected_unit_spans();
        if selected_tokens.is_empty() {
            self.status_message = "No unit selected".into();
            return Ok(());
        }

        let mut replacements = Vec::new();
        for note in selected_tokens {
            if !note.kind.is_pitch() {
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
                self.status_message = "Selected unit no longer exists".into();
                return Ok(());
            };
            replace_char_range(line, note.start_col, note.end_col, new_token);
        }

        self.apply_unit_selection_update(
            lines,
            &selected_indices,
            format!(
                "Transposed {} note{} by {:+}",
                replacements.len(),
                if replacements.len() == 1 { "" } else { "s" },
                semitones
            ),
            audition,
        )
    }

    pub(crate) fn transpose_current_template_call(&mut self, semitones: i32) -> Result<()> {
        let Some(call) = self.current_template_call_at_cursor() else {
            self.status_message = "No template call on this line".into();
            return Ok(());
        };
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(call.row) else {
            self.status_message = "Selected template call no longer exists".into();
            return Ok(());
        };
        let Some(replacement) =
            super::template::transposed_template_call_text(&call.raw_text, semitones)
        else {
            self.status_message = "Template call transpose needs a valid call".into();
            return Ok(());
        };
        replace_char_range(line, call.start_col, call.end_col, &replacement);
        self.apply_cursor_source_update(
            lines,
            (call.row, call.start_col),
            format!(
                "Transposed template call @{} by {:+}",
                call.template_name, semitones
            ),
            None,
        )
    }

    pub(crate) fn transpose_selected_template_calls(&mut self, semitones: i32) -> Result<()> {
        let selected_indices = self.selected_template_call_indices();
        let mut selected = self.selected_template_call_spans();
        if selected.is_empty() {
            self.status_message = "No template call selected".into();
            return Ok(());
        }

        selected.sort_by(|left, right| {
            right
                .row
                .cmp(&left.row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });

        let mut lines = self.textarea.lines().to_vec();
        for span in &selected {
            let Some(line) = lines.get_mut(span.row) else {
                self.status_message = "Selected template call no longer exists".into();
                return Ok(());
            };
            let Some(replacement) =
                super::template::transposed_template_call_text(&span.raw_text, semitones)
            else {
                self.status_message = "Template call transpose needs a valid call".into();
                return Ok(());
            };
            replace_char_range(line, span.start_col, span.end_col, &replacement);
        }

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_template_call_selection_from_indices(&selected_indices);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Transposed {} template call{} by {:+}",
            selected_indices.len(),
            if selected_indices.len() == 1 { "" } else { "s" },
            semitones
        );
        Ok(())
    }
}
