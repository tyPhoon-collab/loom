use crate::interface::studio::selection::{
    char_range, delete_unit, insert_at_col, is_lane_body_token, is_seq_line, replace_char_range,
    StudioSelection,
};
use crate::interface::studio::template_ops::template_call_spans_in_line;
use crate::interface::studio::{StudioApp, StudioMode, UnitYankContext, YankBuffer, YankedBarRow};
use miette::Result;
use ratatui_textarea::CursorMove;

impl StudioApp {
    pub(crate) fn delete_current_unit(&mut self) -> Result<()> {
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

    pub(crate) fn replace_selected_units(&mut self, replacement: &str) -> Result<()> {
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

    pub(crate) fn delete_selected_units(&mut self) -> Result<()> {
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

    pub(crate) fn delete_selection(&mut self) -> Result<()> {
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

    pub(crate) fn delete_selected_bars(&mut self) -> Result<()> {
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

    pub(crate) fn yank_selection(&mut self) {
        if matches!(
            self.selection,
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. })
        ) {
            self.yank_selected_bars();
        } else if matches!(
            self.selection,
            Some(StudioSelection::TemplateCall { .. } | StudioSelection::TemplateCallRange { .. })
        ) {
            self.yank_selected_template_calls();
        } else {
            self.yank_selected_units();
        }
    }

    pub(crate) fn paste_after(&mut self) -> Result<()> {
        let Some(buffer) = self.yank_buffer.clone() else {
            self.status_message = "Nothing yanked".into();
            return Ok(());
        };

        if matches!(
            self.selection,
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. })
        ) {
            match buffer {
                YankBuffer::Bars { rows } => self.paste_yanked_bars_after_selection(rows),
                _ => {
                    self.status_message = "Current selection expects yanked bars".into();
                    Ok(())
                }
            }
        } else if matches!(
            self.selection,
            Some(StudioSelection::TemplateCall { .. } | StudioSelection::TemplateCallRange { .. })
        ) {
            match buffer {
                YankBuffer::TemplateCalls { calls } => {
                    self.paste_yanked_template_calls_after_selection(calls)
                }
                _ => {
                    self.status_message = "Current selection expects yanked template calls".into();
                    Ok(())
                }
            }
        } else {
            match buffer {
                YankBuffer::Units { tokens, context } => {
                    if self.selection.is_some() {
                        self.paste_yanked_units_after_selection(tokens, context)
                    } else {
                        self.paste_yanked_units_after_cursor(tokens, context)
                    }
                }
                YankBuffer::Bars { rows } => {
                    if self.selection.is_some() {
                        self.status_message = "Current selection expects yanked units".into();
                        Ok(())
                    } else {
                        self.paste_yanked_bars_after_cursor(rows)
                    }
                }
                YankBuffer::TemplateCalls { calls } => {
                    if self.selection.is_some() {
                        self.status_message = "Current selection expects yanked units".into();
                        Ok(())
                    } else {
                        self.paste_yanked_template_calls_after_cursor(calls)
                    }
                }
            }
        }
    }

    pub(crate) fn delete_selected_template_calls(&mut self) -> Result<()> {
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

    pub(crate) fn yank_selected_units(&mut self) {
        let selected_indices = self.selected_unit_indices();
        let selected_tokens = self.selected_unit_spans();
        if selected_tokens.is_empty() {
            self.status_message = "y applies to unit selection only".into();
            return;
        }

        let row = selected_tokens[0].row;
        if selected_tokens.iter().any(|note| note.row != row) {
            self.status_message = "Yank currently supports one line at a time".into();
            return;
        }

        let Some(line) = self.textarea.lines().get(row) else {
            self.status_message = "Selected unit no longer exists".into();
            return;
        };

        let context = if is_seq_line(line) {
            UnitYankContext::Seq
        } else if selected_tokens.iter().all(|note| note.kind.is_modifier()) {
            UnitYankContext::Modifier
        } else if selected_tokens
            .iter()
            .all(|note| is_lane_body_token(line, note))
        {
            UnitYankContext::LaneBody
        } else {
            self.status_message =
                "Yank currently supports seq, modifier, or lane body units only".into();
            return;
        };

        self.yank_buffer = Some(YankBuffer::Units {
            tokens: selected_tokens
                .iter()
                .map(|note| note.token.clone())
                .collect::<Vec<_>>(),
            context,
        });
        self.status_message = format!(
            "Yanked {} unit{}",
            selected_indices.len(),
            if selected_indices.len() == 1 { "" } else { "s" }
        );
    }

    pub(crate) fn paste_yanked_units_after_selection(
        &mut self,
        tokens: Vec<String>,
        context: UnitYankContext,
    ) -> Result<()> {
        let selected_indices = self.selected_unit_indices();
        let selected_tokens = self.selected_unit_spans();
        if selected_tokens.is_empty() {
            self.status_message = "p applies to unit selection only".into();
            return Ok(());
        }

        let row = selected_tokens[0].row;
        if selected_tokens.iter().any(|note| note.row != row) {
            self.status_message = "Paste currently supports one line at a time".into();
            return Ok(());
        }

        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(row) else {
            self.status_message = "Selected unit no longer exists".into();
            return Ok(());
        };
        if !unit_yank_context_matches(context, line, &selected_tokens) {
            self.status_message = "Yanked units do not fit the current selection".into();
            return Ok(());
        }
        let Some(last_token) = selected_tokens.last() else {
            return Ok(());
        };

        let insertion = format!(" {}", tokens.join(" "));
        insert_at_col(line, last_token.end_col, &insertion);

        let Some(last_selected_index) = selected_indices.iter().max().copied() else {
            self.status_message = "Selected unit no longer exists".into();
            return Ok(());
        };
        let inserted_indices: Vec<usize> =
            (last_selected_index + 1..=last_selected_index + tokens.len()).collect();

        let audition = self.audition_candidate_from_indices(&inserted_indices);
        self.apply_unit_selection_update(
            lines,
            &inserted_indices,
            format!(
                "Pasted {} unit{}",
                tokens.len(),
                if tokens.len() == 1 { "" } else { "s" }
            ),
            audition,
        )
    }

    pub(crate) fn paste_yanked_units_after_cursor(
        &mut self,
        tokens: Vec<String>,
        context: UnitYankContext,
    ) -> Result<()> {
        let cursor = self.textarea.cursor();
        let Some(token) = self.unit_at_or_after_cursor(cursor.0, cursor.1) else {
            self.status_message = "Paste needs the cursor on a unit".into();
            return Ok(());
        };

        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(token.row) else {
            self.status_message = "Selected unit no longer exists".into();
            return Ok(());
        };
        if !unit_yank_context_matches(context, line, std::slice::from_ref(&token)) {
            self.status_message = "Yanked units do not fit the current cursor target".into();
            return Ok(());
        }

        let insertion = format!(" {}", tokens.join(" "));
        insert_at_col(line, token.end_col, &insertion);

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        if let Some(inserted) = self.unit_at_or_after_cursor(token.row, token.end_col + 1) {
            self.textarea.move_cursor(CursorMove::Jump(
                inserted.row as u16,
                inserted.start_col as u16,
            ));
        }
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Pasted {} unit{}",
            tokens.len(),
            if tokens.len() == 1 { "" } else { "s" }
        );
        Ok(())
    }

    pub(crate) fn yank_selected_bars(&mut self) {
        let selected_bars = self.selected_bar_spans();
        if selected_bars.is_empty() {
            self.status_message = "y applies to bar selection only".into();
            return;
        }

        let mut selected_by_row = std::collections::BTreeMap::<
            usize,
            Vec<crate::interface::studio::selection::BarSpan>,
        >::new();
        for bar in selected_bars {
            selected_by_row.entry(bar.row).or_default().push(bar);
        }
        let yanked_bar_count: usize = selected_by_row.values().map(Vec::len).sum();
        let mut rows = Vec::new();

        for (row, mut row_bars) in selected_by_row {
            row_bars.sort_by_key(|bar| bar.index);
            let Some(first) = row_bars.first().cloned() else {
                continue;
            };
            let Some(last) = row_bars.last().cloned() else {
                continue;
            };
            let Some(line) = self.textarea.lines().get(row) else {
                self.status_message = "Selected bar no longer exists".into();
                return;
            };

            rows.push(YankedBarRow {
                text: char_range(line, first.start_col + 1, last.end_col),
                count: row_bars.len(),
            });
        }

        self.yank_buffer = Some(YankBuffer::Bars { rows });
        self.status_message = format!(
            "Yanked {} bar{}",
            yanked_bar_count,
            if yanked_bar_count == 1 { "" } else { "s" }
        );
    }

    pub(crate) fn paste_yanked_bars_after_selection(
        &mut self,
        rows: Vec<YankedBarRow>,
    ) -> Result<()> {
        let selected_bars = self.selected_bar_spans();
        if selected_bars.is_empty() {
            self.status_message = "p applies to bar selection only".into();
            return Ok(());
        }

        let mut selected_by_row = std::collections::BTreeMap::<
            usize,
            Vec<crate::interface::studio::selection::BarSpan>,
        >::new();
        for bar in selected_bars {
            selected_by_row.entry(bar.row).or_default().push(bar);
        }

        if selected_by_row.len() != rows.len() {
            self.status_message = "Yanked bars need the same number of selected rows".into();
            return Ok(());
        }

        let mut lines = self.textarea.lines().to_vec();
        let mut inserted_positions = Vec::new();
        let mut pasted_bar_count = 0usize;

        for ((row, mut row_bars), yanked_row) in selected_by_row
            .into_iter()
            .rev()
            .zip(rows.into_iter().rev())
        {
            row_bars.sort_by_key(|bar| bar.index);
            let Some(last) = row_bars.last().cloned() else {
                continue;
            };
            let Some(line) = lines.get_mut(row) else {
                self.status_message = "Selected bar no longer exists".into();
                return Ok(());
            };

            insert_at_col(line, last.end_col, &yanked_row.text);
            inserted_positions
                .extend((last.index + 1..=last.index + yanked_row.count).map(|index| (row, index)));
            pasted_bar_count += yanked_row.count;
        }

        inserted_positions.sort_unstable();
        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_bar_selection_from_positions(&inserted_positions);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Pasted {} bar{}",
            pasted_bar_count,
            if pasted_bar_count == 1 { "" } else { "s" }
        );
        Ok(())
    }

    pub(crate) fn paste_yanked_bars_after_cursor(&mut self, rows: Vec<YankedBarRow>) -> Result<()> {
        let [yanked_row] = rows.as_slice() else {
            self.status_message = "Multi-row bar paste needs bar selection".into();
            return Ok(());
        };
        let cursor = self.textarea.cursor();
        let Some(bar) = self.bar_at_or_after_cursor(cursor.0, cursor.1) else {
            self.status_message = "Paste needs the cursor on a bar".into();
            return Ok(());
        };

        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(bar.row) else {
            self.status_message = "Selected bar no longer exists".into();
            return Ok(());
        };
        insert_at_col(line, bar.end_col, &yanked_row.text);

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        if let Some(inserted) = self.bar_at_or_after_cursor(bar.row, bar.end_col + 1) {
            self.focus_bar_cursor(&inserted);
        }
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Pasted {} bar{}",
            yanked_row.count,
            if yanked_row.count == 1 { "" } else { "s" }
        );
        Ok(())
    }

    pub(crate) fn yank_selected_template_calls(&mut self) {
        let selected = self.selected_template_call_spans();
        if selected.is_empty() {
            self.status_message = "y applies to template call selection only".into();
            return;
        }

        let row = selected[0].row;
        if selected.iter().any(|span| span.row != row) {
            self.status_message = "Yank currently supports one template line at a time".into();
            return;
        }

        self.yank_buffer = Some(YankBuffer::TemplateCalls {
            calls: selected
                .iter()
                .map(|span| span.raw_text.clone())
                .collect::<Vec<_>>(),
        });
        self.status_message = format!(
            "Yanked {} template call{}",
            selected.len(),
            if selected.len() == 1 { "" } else { "s" }
        );
    }

    pub(crate) fn paste_yanked_template_calls_after_selection(
        &mut self,
        calls: Vec<String>,
    ) -> Result<()> {
        let selected = self.selected_template_call_spans();
        if selected.is_empty() {
            self.status_message = "p applies to template call selection only".into();
            return Ok(());
        }

        let row = selected[0].row;
        if selected.iter().any(|span| span.row != row) {
            self.status_message = "Paste currently supports one template line at a time".into();
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
        let insertion = format!(" {}", calls.join(" "));
        insert_at_col(line, last.end_col, &insertion);

        let inserted_positions: Vec<(usize, usize)> =
            template_call_positions_after_duplication(row, line, calls.len());
        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_template_call_selection_from_positions(&inserted_positions);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Pasted {} template call{}",
            calls.len(),
            if calls.len() == 1 { "" } else { "s" }
        );
        Ok(())
    }

    pub(crate) fn paste_yanked_template_calls_after_cursor(
        &mut self,
        calls: Vec<String>,
    ) -> Result<()> {
        let Some(call) = self.current_template_call_at_cursor() else {
            self.status_message = "Paste needs the cursor on a template call".into();
            return Ok(());
        };

        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(call.row) else {
            self.status_message = "Selected template call no longer exists".into();
            return Ok(());
        };
        let insertion = format!(" {}", calls.join(" "));
        insert_at_col(line, call.end_col, &insertion);

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        if let Some(inserted) = self.template_call_at_or_after_cursor(call.row, call.end_col + 1) {
            self.textarea.move_cursor(CursorMove::Jump(
                inserted.row as u16,
                inserted.start_col as u16,
            ));
        }
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Pasted {} template call{}",
            calls.len(),
            if calls.len() == 1 { "" } else { "s" }
        );
        Ok(())
    }
}

fn unit_yank_context_matches(
    context: UnitYankContext,
    line: &str,
    target_tokens: &[crate::interface::studio::selection::UnitSpan],
) -> bool {
    match context {
        UnitYankContext::Seq => is_seq_line(line),
        UnitYankContext::Modifier => target_tokens.iter().all(|note| note.kind.is_modifier()),
        UnitYankContext::LaneBody => target_tokens
            .iter()
            .all(|note| is_lane_body_token(line, note)),
    }
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
    let spans = template_call_spans_in_line(row, line);
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
