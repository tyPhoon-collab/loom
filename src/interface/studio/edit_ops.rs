use super::selection::{
    bar_spans_in_line, char_range, delete_unit, group_span_containing_col, insert_at_col,
    is_lane_body_token, is_seq_line, replace_char_range, unit_spans_in_line, GroupSpan,
    StudioSelection,
};
use super::transform::{transpose_bar_text, transpose_line, transpose_note_token};
use super::{StudioApp, StudioMode};
use miette::Result;
use ratatui_textarea::CursorMove;

impl StudioApp {
    pub(super) fn delete_current_unit(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let Some(token) = self.unit_at_or_after_cursor(cursor.0, cursor.1) else {
            self.status_message = "No unit on this line".into();
            return Ok(());
        };

        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(token.row) else {
            self.status_message = "Selected unit no longer exists".into();
            return Ok(());
        };
        delete_unit(line, &token);

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.textarea
            .move_cursor(CursorMove::Jump(token.row as u16, token.start_col as u16));
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!("Deleted unit {}", token.token);
        Ok(())
    }

    pub(super) fn subdivide_current_unit(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let Some(token) = self.unit_at_or_after_cursor(cursor.0, cursor.1) else {
            self.status_message = "No unit on this line".into();
            return Ok(());
        };

        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(token.row) else {
            self.status_message = "Selected unit no longer exists".into();
            return Ok(());
        };
        if !is_subdividable_token(line, &token) {
            self.status_message = "Subdivide needs a seq or lane body token".into();
            return Ok(());
        }

        replace_char_range(
            line,
            token.start_col,
            token.end_col,
            &subdivided_unit(&token.token),
        );
        let cursor_col = unit_spans_in_line(token.row, line)
            .into_iter()
            .find(|span| span.start_col >= token.start_col)
            .map(|span| span.start_col)
            .unwrap_or(token.start_col);

        self.apply_cursor_source_update(
            lines,
            (token.row, cursor_col),
            "Subdivided unit".into(),
            None,
        )
    }

    pub(super) fn subdivide_selected_units(&mut self) -> Result<()> {
        let selected_indices = self.selected_unit_indices();
        let mut selected_tokens = self.selected_unit_spans();
        if selected_tokens.is_empty() {
            self.status_message = "Subdivide applies to unit selection only".into();
            return Ok(());
        }

        let lines = self.textarea.lines();
        if selected_tokens.iter().any(|selected| {
            lines
                .get(selected.row)
                .is_none_or(|line| !is_subdividable_token(line, selected))
        }) {
            self.status_message =
                "Subdivide applies to seq or lane body unit selection only".into();
            return Ok(());
        }

        selected_tokens.sort_by(|left, right| {
            right
                .row
                .cmp(&left.row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });

        let mut lines = lines.to_vec();
        for selected in &selected_tokens {
            let Some(line) = lines.get_mut(selected.row) else {
                self.status_message = "Selected unit no longer exists".into();
                return Ok(());
            };
            replace_char_range(
                line,
                selected.start_col,
                selected.end_col,
                &subdivided_unit(&selected.token),
            );
        }

        let Some(&first_index) = selected_indices.first() else {
            self.status_message = "Subdivide applies to unit selection only".into();
            return Ok(());
        };
        let expanded_indices: Vec<usize> =
            (first_index..=first_index + selected_indices.len() * 2 - 1).collect();

        self.apply_unit_selection_update(
            lines,
            &expanded_indices,
            format!(
                "Subdivided {} unit{}",
                selected_indices.len(),
                if selected_indices.len() == 1 { "" } else { "s" }
            ),
            None,
        )
    }

    pub(super) fn shrink_current_editable_group(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let Some(token) = self.unit_at_or_after_cursor(cursor.0, cursor.1) else {
            self.status_message = "No unit on this line".into();
            return Ok(());
        };

        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(token.row) else {
            self.status_message = "Selected unit no longer exists".into();
            return Ok(());
        };
        let Some(group) = shrinkable_group_at_token(line, &token) else {
            self.status_message = "Shrink needs a bracket group".into();
            return Ok(());
        };

        replace_char_range(
            line,
            group.start_col,
            group.end_col,
            &group.selected_element,
        );
        self.apply_cursor_source_update(
            lines,
            (token.row, group.start_col),
            "Shrank group to selected element".into(),
            None,
        )
    }

    pub(super) fn shrink_selected_editable_groups(&mut self) -> Result<()> {
        let mut selected_tokens = self.selected_unit_spans();
        if selected_tokens.is_empty() {
            self.status_message = "Shrink applies to unit selection only".into();
            return Ok(());
        }
        selected_tokens.sort_by(|left, right| {
            left.row
                .cmp(&right.row)
                .then_with(|| left.start_col.cmp(&right.start_col))
        });

        let lines = self.textarea.lines();
        let mut groups = Vec::<(usize, GroupSpan)>::new();
        for token in &selected_tokens {
            let Some(line) = lines.get(token.row) else {
                self.status_message = "Selected unit no longer exists".into();
                return Ok(());
            };
            let Some(group) = shrinkable_group_at_token(line, token) else {
                self.status_message = "Shrink needs bracket groups".into();
                return Ok(());
            };
            if !groups
                .iter()
                .any(|(row, existing)| *row == token.row && existing.start_col == group.start_col)
            {
                groups.push((token.row, group));
            }
        }
        if groups.is_empty() {
            self.status_message = "Shrink needs bracket groups".into();
            return Ok(());
        }

        groups.sort_by(|(left_row, left), (right_row, right)| {
            right_row
                .cmp(left_row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });

        let mut lines = lines.to_vec();
        let mut selection_positions = Vec::new();
        for (row, group) in &groups {
            let Some(line) = lines.get_mut(*row) else {
                self.status_message = "Selected unit no longer exists".into();
                return Ok(());
            };
            replace_char_range(
                line,
                group.start_col,
                group.end_col,
                &group.selected_element,
            );
            selection_positions.push((*row, group.start_col));
        }
        selection_positions.sort_unstable();

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_unit_selection_from_positions(&selection_positions);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Shrank {} group{} to selected element",
            selection_positions.len(),
            if selection_positions.len() == 1 {
                ""
            } else {
                "s"
            }
        );
        Ok(())
    }

    pub(super) fn transpose_selection(&mut self, semitones: i32) -> Result<()> {
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

    pub(super) fn transpose_current_unit(&mut self, semitones: i32) -> Result<()> {
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

    pub(super) fn transpose_selected_units(&mut self, semitones: i32) -> Result<()> {
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

    pub(super) fn transpose_current_template_call(&mut self, semitones: i32) -> Result<()> {
        let Some(call) = self.current_template_call_at_cursor() else {
            self.status_message = "No template call on this line".into();
            return Ok(());
        };
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(call.row) else {
            self.status_message = "Selected template call no longer exists".into();
            return Ok(());
        };
        let Some(replacement) = transposed_template_call_text(&call.raw_text, semitones) else {
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

    pub(super) fn transpose_selected_template_calls(&mut self, semitones: i32) -> Result<()> {
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
            let Some(replacement) = transposed_template_call_text(&span.raw_text, semitones) else {
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

    pub(super) fn replace_selected_units(&mut self, replacement: &str) -> Result<()> {
        let selected_indices = self.selected_unit_indices();
        let mut selected_tokens = self.selected_unit_spans();
        if selected_tokens.is_empty() {
            self.status_message = "Replacement applies to unit selection only".into();
            return Ok(());
        }
        let lines = self.textarea.lines();
        if replacement != "."
            && replacement != "-"
            && selected_tokens.iter().any(|note| {
                lines
                    .get(note.row)
                    .is_some_and(|line| is_lane_body_token(line, note))
            })
        {
            self.status_message = "Note entry does not apply to lane body units".into();
            return Ok(());
        }
        if selected_tokens.iter().any(|note| note.kind.is_modifier()) {
            self.status_message = "Note entry does not apply to modifier units".into();
            return Ok(());
        }

        let audition = (replacement != "." && replacement != "-")
            .then(|| {
                selected_tokens
                    .first()
                    .map(|note| (note.row, replacement.to_string()))
            })
            .flatten();

        let replacement_name = match replacement {
            "." => "Rested",
            "-" => "Sustained",
            _ => "Replaced",
        };
        selected_tokens.sort_by(|left, right| {
            right
                .row
                .cmp(&left.row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });

        let mut lines = self.textarea.lines().to_vec();
        for note in &selected_tokens {
            let Some(line) = lines.get_mut(note.row) else {
                self.status_message = "Selected unit no longer exists".into();
                return Ok(());
            };
            replace_char_range(line, note.start_col, note.end_col, replacement);
        }

        self.apply_unit_selection_update(
            lines,
            &selected_indices,
            format!(
                "{} {} unit{}",
                replacement_name,
                selected_tokens.len(),
                if selected_tokens.len() == 1 { "" } else { "s" }
            ),
            audition,
        )
    }

    pub(super) fn delete_selected_units(&mut self) -> Result<()> {
        let mut selected_tokens = self.selected_unit_spans();
        if selected_tokens.is_empty() {
            self.status_message = "x applies to unit selection only".into();
            return Ok(());
        }

        let first = selected_tokens.first().cloned();
        selected_tokens.sort_by(|left, right| {
            right
                .row
                .cmp(&left.row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });

        let mut lines = self.textarea.lines().to_vec();
        for note in &selected_tokens {
            let Some(line) = lines.get_mut(note.row) else {
                self.status_message = "Selected unit no longer exists".into();
                return Ok(());
            };
            delete_unit(line, note);
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
            "Deleted {} unit{}",
            selected_tokens.len(),
            if selected_tokens.len() == 1 { "" } else { "s" }
        );
        Ok(())
    }

    pub(super) fn delete_selection(&mut self) -> Result<()> {
        if matches!(
            self.selection,
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. })
        ) {
            self.delete_selected_bars()
        } else if matches!(
            self.selection,
            Some(StudioSelection::TemplateCall { .. } | StudioSelection::TemplateCallRange { .. })
        ) {
            self.delete_selected_template_calls()
        } else {
            self.delete_selected_units()
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
        } else if matches!(
            self.selection,
            Some(StudioSelection::TemplateCall { .. } | StudioSelection::TemplateCallRange { .. })
        ) {
            self.duplicate_selected_template_calls()
        } else {
            self.duplicate_selected_units()
        }
    }

    pub(super) fn delete_selected_template_calls(&mut self) -> Result<()> {
        let mut selected = self.selected_template_call_spans();
        if selected.is_empty() {
            self.status_message = "x applies to template call selection only".into();
            return Ok(());
        }

        let first = selected.first().cloned();
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
            delete_template_call(line, span.start_col, span.end_col);
        }

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        if let Some(span) = first {
            self.textarea
                .move_cursor(CursorMove::Jump(span.row as u16, span.start_col as u16));
        }
        self.selection = None;
        self.textarea.cancel_selection();
        self.mode = StudioMode::Normal;
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Deleted {} template call{}",
            selected.len(),
            if selected.len() == 1 { "" } else { "s" }
        );
        Ok(())
    }

    pub(super) fn duplicate_selected_units(&mut self) -> Result<()> {
        let selected_indices = self.selected_unit_indices();
        let selected_tokens = self.selected_unit_spans();
        if selected_tokens.is_empty() {
            self.status_message = "d applies to unit selection only".into();
            return Ok(());
        }

        let row = selected_tokens[0].row;
        if selected_tokens.iter().any(|note| note.row != row) {
            self.status_message = "Duplicate currently supports one seq line at a time".into();
            return Ok(());
        }

        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(row) else {
            self.status_message = "Selected unit no longer exists".into();
            return Ok(());
        };
        let Some(last_token) = selected_tokens.last() else {
            return Ok(());
        };
        if !is_seq_line(line)
            && !selected_tokens.iter().all(|note| note.kind.is_modifier())
            && !selected_tokens
                .iter()
                .all(|note| is_lane_body_token(line, note))
        {
            self.status_message =
                "Duplicate currently supports seq, modifier, or lane body units only".into();
            return Ok(());
        }

        let insertion = format!(
            " {}",
            selected_tokens
                .iter()
                .map(|note| note.token.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        );
        insert_at_col(line, last_token.end_col, &insertion);

        let Some(last_selected_index) = selected_indices.iter().max().copied() else {
            self.status_message = "Selected unit no longer exists".into();
            return Ok(());
        };
        let inserted_indices: Vec<usize> =
            (last_selected_index + 1..=last_selected_index + selected_tokens.len()).collect();

        let audition = self.audition_candidate_from_indices(&inserted_indices);
        self.apply_unit_selection_update(
            lines,
            &inserted_indices,
            format!(
                "Duplicated {} unit{}",
                selected_tokens.len(),
                if selected_tokens.len() == 1 { "" } else { "s" }
            ),
            audition,
        )
    }

    pub(super) fn duplicate_selected_bars(&mut self) -> Result<()> {
        let selected_bars = self.selected_bar_spans();
        if selected_bars.is_empty() {
            self.status_message = "d applies to bar selection only".into();
            return Ok(());
        }

        let mut lines = self.textarea.lines().to_vec();
        let mut inserted_positions = Vec::new();
        let mut selected_by_row =
            std::collections::BTreeMap::<usize, Vec<super::selection::BarSpan>>::new();
        for bar in selected_bars {
            selected_by_row.entry(bar.row).or_default().push(bar);
        }
        let duplicated_bar_count: usize = selected_by_row.values().map(Vec::len).sum();

        for (row, mut row_bars) in selected_by_row.into_iter().rev() {
            row_bars.sort_by_key(|bar| bar.index);
            let Some(first) = row_bars.first().cloned() else {
                continue;
            };
            let Some(last) = row_bars.last().cloned() else {
                continue;
            };
            let Some(line) = lines.get_mut(row) else {
                self.status_message = "Selected bar no longer exists".into();
                return Ok(());
            };

            let insertion = char_range(line, first.start_col + 1, last.end_col);
            insert_at_col(line, last.end_col, &insertion);
            inserted_positions
                .extend((last.index + 1..=last.index + row_bars.len()).map(|index| (row, index)));
        }

        inserted_positions.sort_unstable();
        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_bar_selection_from_positions(&inserted_positions);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Duplicated {} bar{}",
            duplicated_bar_count,
            if duplicated_bar_count == 1 { "" } else { "s" }
        );
        Ok(())
    }

    pub(super) fn duplicate_selected_template_calls(&mut self) -> Result<()> {
        let selected = self.selected_template_call_spans();
        if selected.is_empty() {
            self.status_message = "d applies to template call selection only".into();
            return Ok(());
        }

        let row = selected[0].row;
        if selected.iter().any(|span| span.row != row) {
            self.status_message = "Duplicate currently supports one template line at a time".into();
            return Ok(());
        }

        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(row) else {
            self.status_message = "Selected template call no longer exists".into();
            return Ok(());
        };
        let Some(last) = selected.last() else {
            return Ok(());
        };
        let insertion = format!(
            " {}",
            selected
                .iter()
                .map(|span| span.raw_text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        );
        insert_at_col(line, last.end_col, &insertion);

        let inserted_positions: Vec<(usize, usize)> =
            template_call_positions_after_duplication(row, line, selected.len());
        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_template_call_selection_from_positions(&inserted_positions);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Duplicated {} template call{}",
            selected.len(),
            if selected.len() == 1 { "" } else { "s" }
        );
        Ok(())
    }
}

fn is_subdividable_token(line: &str, token: &super::selection::UnitSpan) -> bool {
    token.kind.is_seq_body() || (is_lane_body_token(line, token) && !is_seq_line(line))
}

fn subdivided_unit(token: &str) -> String {
    format!("[{} .]", token)
}

fn shrinkable_group_at_token(line: &str, token: &super::selection::UnitSpan) -> Option<GroupSpan> {
    group_span_containing_col(line, token.start_col)
}

fn delete_template_call(line: &mut String, start_col: usize, end_col: usize) {
    let chars: Vec<char> = line.chars().collect();
    let mut start = start_col;
    let mut end = end_col;

    while end < chars.len() && chars[end].is_whitespace() {
        end += 1;
    }
    if end == end_col {
        while start > 0 && chars[start - 1].is_whitespace() {
            start -= 1;
        }
    }

    replace_char_range(line, start, end, "");
}

fn template_call_positions_after_duplication(
    row: usize,
    line: &str,
    duplicated_count: usize,
) -> Vec<(usize, usize)> {
    let spans = super::template_ops::template_call_spans_in_line(row, line);
    spans
        .iter()
        .rev()
        .take(duplicated_count)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|span| (span.row, span.start_col))
        .collect()
}

fn transposed_template_call_text(raw_text: &str, semitones: i32) -> Option<String> {
    let body = raw_text.strip_prefix("[@")?;
    let closing = body.find(']')?;
    let inside = &body[..closing];
    let suffix = &body[closing + 1..];

    let mut parts = inside.split_whitespace();
    let name = parts.next()?;
    let params: Vec<&str> = parts.collect();

    let mut transpose_slot = None;
    let mut current = 0i32;
    let mut rebuilt = Vec::with_capacity(params.len() + 1);

    for param in params {
        if let Some(value) = parse_template_call_transpose(param) {
            if transpose_slot.is_none() {
                transpose_slot = Some(rebuilt.len());
                current = value;
                rebuilt.push(String::new());
            }
        } else {
            rebuilt.push(param.to_string());
        }
    }

    let updated = current + semitones;
    match transpose_slot {
        Some(index) if updated == 0 => {
            rebuilt.remove(index);
        }
        Some(index) => {
            rebuilt[index] = format_template_call_transpose(updated);
        }
        None if updated != 0 => {
            rebuilt.insert(0, format_template_call_transpose(updated));
        }
        None => {}
    }

    let mut out = format!("[@{}", name);
    for param in rebuilt {
        if !param.is_empty() {
            out.push(' ');
            out.push_str(&param);
        }
    }
    out.push(']');
    out.push_str(suffix);
    Some(out)
}

fn parse_template_call_transpose(input: &str) -> Option<i32> {
    let (sign, digits) = input.split_at(1);
    let value = digits.parse::<i32>().ok()?;
    match sign {
        "+" => Some(value),
        "-" => Some(-value),
        _ => None,
    }
}

fn format_template_call_transpose(value: i32) -> String {
    if value >= 0 {
        format!("+{}", value)
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        format_template_call_transpose, parse_template_call_transpose, shrinkable_group_at_token,
        subdivided_unit, transposed_template_call_text,
    };
    use crate::interface::studio::selection::{replace_char_range, unit_spans_in_line};

    #[test]
    fn subdivided_unit_wraps_seq_note() {
        assert_eq!(subdivided_unit("C4"), "[C4 .]");
        assert_eq!(subdivided_unit("."), "[. .]");
        assert_eq!(subdivided_unit("-"), "[- .]");
    }

    #[test]
    fn subdivided_unit_replaces_seq_slot() {
        let mut line = "seq | C4 . |".to_string();
        let token = unit_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "C4")
            .unwrap();
        replace_char_range(
            &mut line,
            token.start_col,
            token.end_col,
            &subdivided_unit(&token.token),
        );
        assert_eq!(line, "seq | [C4 .] . |");
    }

    #[test]
    fn shrinkable_group_at_token_uses_selected_element() {
        let line = "seq | [C4 .] . |".to_string();
        let token = unit_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "C4")
            .unwrap();
        let group = shrinkable_group_at_token(&line, &token).unwrap();
        assert_eq!(group.selected_element, "C4");
    }

    #[test]
    fn shrinkable_group_at_token_uses_nearest_group_element() {
        let line = "seq | [[C4 .] [. .]] . |".to_string();
        let token = unit_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "C4")
            .unwrap();
        let group = shrinkable_group_at_token(&line, &token).unwrap();
        assert_eq!(group.selected_element, "C4");
        assert_eq!((group.start_col, group.end_col), (7, 13));
    }

    #[test]
    fn template_call_transpose_adds_new_param() {
        assert_eq!(
            transposed_template_call_text("[@riff arp]*2", 12).as_deref(),
            Some("[@riff +12 arp]*2")
        );
    }

    #[test]
    fn template_call_transpose_updates_existing_param() {
        assert_eq!(
            transposed_template_call_text("[@riff +12 arp]", -1).as_deref(),
            Some("[@riff +11 arp]")
        );
    }

    #[test]
    fn template_call_transpose_removes_zero_param() {
        assert_eq!(
            transposed_template_call_text("[@riff +12 rev]", -12).as_deref(),
            Some("[@riff rev]")
        );
    }

    #[test]
    fn parse_and_format_template_call_transpose_roundtrip() {
        assert_eq!(parse_template_call_transpose("+12"), Some(12));
        assert_eq!(parse_template_call_transpose("-7"), Some(-7));
        assert_eq!(format_template_call_transpose(5), "+5");
        assert_eq!(format_template_call_transpose(-5), "-5");
    }
}
