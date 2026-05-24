use crate::interface::studio::selection::{
    group_span_containing_col, is_lane_body_token, is_seq_line, replace_char_range,
    unit_spans_in_line, GroupSpan,
};
use crate::interface::studio::StudioApp;
use miette::Result;

impl StudioApp {
    pub(crate) fn subdivide_current_unit(&mut self) -> Result<()> {
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

    pub(crate) fn subdivide_selected_units(&mut self) -> Result<()> {
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

    pub(crate) fn shrink_current_editable_group(&mut self) -> Result<()> {
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

    pub(crate) fn shrink_selected_editable_groups(&mut self) -> Result<()> {
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
}

pub(super) fn is_subdividable_token(
    line: &str,
    token: &crate::interface::studio::selection::UnitSpan,
) -> bool {
    token.kind.is_seq_body() || (is_lane_body_token(line, token) && !is_seq_line(line))
}

pub(super) fn subdivided_unit(token: &str) -> String {
    format!("[{} .]", token)
}

pub(super) fn shrinkable_group_at_token(
    line: &str,
    token: &crate::interface::studio::selection::UnitSpan,
) -> Option<GroupSpan> {
    group_span_containing_col(line, token.start_col)
}
