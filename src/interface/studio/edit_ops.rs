use super::selection::{
    bar_spans_in_line, char_range, delete_editable_token, editable_token_spans_in_line,
    insert_at_col, is_lane_body_token, is_seq_line, replace_char_range, EditableTokenKind,
    StudioSelection,
};
use super::transform::{transpose_bar_text, transpose_line, transpose_note_token};
use super::{StudioApp, StudioMode};
use miette::Result;
use ratatui_textarea::CursorMove;

impl StudioApp {
    pub(super) fn subdivide_current_editable_token(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let Some(token) = self.editable_token_at_or_after_cursor(cursor.0, cursor.1) else {
            self.status_message = "No editable token on this line".into();
            return Ok(());
        };

        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(token.row) else {
            self.status_message = "Selected token no longer exists".into();
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
            &subdivided_editable_token(&token.token),
        );
        let cursor_col = editable_token_spans_in_line(token.row, line)
            .into_iter()
            .find(|span| span.start_col >= token.start_col)
            .map(|span| span.start_col)
            .unwrap_or(token.start_col);

        self.apply_cursor_source_update(
            lines,
            (token.row, cursor_col),
            "Subdivided token".into(),
            None,
        )
    }

    pub(super) fn subdivide_selected_editable_tokens(&mut self) -> Result<()> {
        let selected_indices = self.selected_editable_token_indices();
        let mut selected_tokens = self.selected_editable_token_spans();
        if selected_tokens.is_empty() {
            self.status_message = "Subdivide applies to editable token selection only".into();
            return Ok(());
        }

        let lines = self.textarea.lines();
        if selected_tokens.iter().any(|selected| {
            lines
                .get(selected.row)
                .is_none_or(|line| !is_subdividable_token(line, selected))
        }) {
            self.status_message =
                "Subdivide applies to seq or lane body token selection only".into();
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
                self.status_message = "Selected token no longer exists".into();
                return Ok(());
            };
            replace_char_range(
                line,
                selected.start_col,
                selected.end_col,
                &subdivided_editable_token(&selected.token),
            );
        }

        let Some(&first_index) = selected_indices.first() else {
            self.status_message = "Subdivide applies to editable token selection only".into();
            return Ok(());
        };
        let expanded_indices: Vec<usize> =
            (first_index..=first_index + selected_indices.len() * 2 - 1).collect();

        self.apply_editable_token_selection_update(
            lines,
            &expanded_indices,
            format!(
                "Subdivided {} token{}",
                selected_indices.len(),
                if selected_indices.len() == 1 { "" } else { "s" }
            ),
            None,
        )
    }

    pub(super) fn transpose_selection(&mut self, semitones: i32) -> Result<()> {
        if matches!(
            self.selection,
            Some(
                StudioSelection::EditableToken { .. } | StudioSelection::EditableTokenRange { .. }
            )
        ) {
            return self.transpose_selected_editable_tokens(semitones);
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
            editable_token_spans_in_line(old_bar.row, line)
                .into_iter()
                .find(|note| {
                    note.kind == EditableTokenKind::Note
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

    pub(super) fn transpose_selected_editable_tokens(&mut self, semitones: i32) -> Result<()> {
        let selected_indices = self.selected_editable_token_indices();
        let selected_tokens = self.selected_editable_token_spans();
        if selected_tokens.is_empty() {
            self.status_message = "No editable token selected".into();
            return Ok(());
        }

        let mut replacements = Vec::new();
        for note in selected_tokens {
            if note.kind != EditableTokenKind::Note {
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

        self.apply_editable_token_selection_update(
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

    pub(super) fn replace_selected_tokens(&mut self, replacement: &str) -> Result<()> {
        let selected_indices = self.selected_editable_token_indices();
        let mut selected_tokens = self.selected_editable_token_spans();
        if selected_tokens.is_empty() {
            self.status_message = "Replacement applies to editable token selection only".into();
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
            self.status_message = "Note entry does not apply to lane body tokens".into();
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
                self.status_message = "Selected token no longer exists".into();
                return Ok(());
            };
            replace_char_range(line, note.start_col, note.end_col, replacement);
        }

        self.apply_editable_token_selection_update(
            lines,
            &selected_indices,
            format!(
                "{} {} token{}",
                replacement_name,
                selected_tokens.len(),
                if selected_tokens.len() == 1 { "" } else { "s" }
            ),
            audition,
        )
    }

    pub(super) fn delete_selected_editable_tokens(&mut self) -> Result<()> {
        let mut selected_tokens = self.selected_editable_token_spans();
        if selected_tokens.is_empty() {
            self.status_message = "x applies to editable token selection only".into();
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
                self.status_message = "Selected token no longer exists".into();
                return Ok(());
            };
            delete_editable_token(line, note);
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
        } else {
            self.delete_selected_editable_tokens()
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
            self.duplicate_selected_editable_tokens()
        }
    }

    pub(super) fn duplicate_selected_editable_tokens(&mut self) -> Result<()> {
        let selected_indices = self.selected_editable_token_indices();
        let selected_tokens = self.selected_editable_token_spans();
        if selected_tokens.is_empty() {
            self.status_message = "d applies to editable token selection only".into();
            return Ok(());
        }

        let row = selected_tokens[0].row;
        if selected_tokens.iter().any(|note| note.row != row) {
            self.status_message = "Duplicate currently supports one seq line at a time".into();
            return Ok(());
        }

        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(row) else {
            self.status_message = "Selected token no longer exists".into();
            return Ok(());
        };
        let Some(last_token) = selected_tokens.last() else {
            return Ok(());
        };
        if !is_seq_line(line)
            && !selected_tokens
                .iter()
                .all(|note| is_lane_body_token(line, note))
        {
            self.status_message =
                "Duplicate currently supports seq body or lane body tokens only".into();
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
            self.status_message = "Selected token no longer exists".into();
            return Ok(());
        };
        let inserted_indices: Vec<usize> =
            (last_selected_index + 1..=last_selected_index + selected_tokens.len()).collect();

        let audition = self.audition_candidate_from_indices(&inserted_indices);
        self.apply_editable_token_selection_update(
            lines,
            &inserted_indices,
            format!(
                "Duplicated {} token{}",
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
}

fn is_subdividable_token(line: &str, token: &super::selection::EditableTokenSpan) -> bool {
    is_seq_line(line) || is_lane_body_token(line, token)
}

fn subdivided_editable_token(token: &str) -> String {
    format!("[{} .]", token)
}

#[cfg(test)]
mod tests {
    use super::subdivided_editable_token;
    use crate::interface::studio::selection::{editable_token_spans_in_line, replace_char_range};

    #[test]
    fn subdivided_editable_token_wraps_seq_note() {
        assert_eq!(subdivided_editable_token("C4"), "[C4 .]");
        assert_eq!(subdivided_editable_token("."), "[. .]");
        assert_eq!(subdivided_editable_token("-"), "[- .]");
    }

    #[test]
    fn subdivided_editable_token_replaces_seq_slot() {
        let mut line = "seq | C4 . |".to_string();
        let token = editable_token_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "C4")
            .unwrap();
        replace_char_range(
            &mut line,
            token.start_col,
            token.end_col,
            &subdivided_editable_token(&token.token),
        );
        assert_eq!(line, "seq | [C4 .] . |");
    }
}
