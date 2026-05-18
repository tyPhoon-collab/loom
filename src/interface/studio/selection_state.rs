use super::selection::{
    bar_at_or_near_col, bar_spans_in_line, editable_token_at_or_near_col,
    editable_token_spans_in_line, next_editable_token_after_position,
    previous_editable_token_before_position, BarSpan, EditableTokenSpan, StudioSelection,
};
use super::StudioApp;
use ratatui_textarea::CursorMove;

impl StudioApp {
    pub(super) fn adjacent_editable_token(
        &self,
        direction: i32,
        row: usize,
        col: usize,
    ) -> Option<EditableTokenSpan> {
        let notes = self.editable_token_spans();
        if direction < 0 {
            previous_editable_token_before_position(&notes, row, col)
        } else {
            next_editable_token_after_position(&notes, row, col)
        }
    }

    pub(super) fn focus_editable_token_cursor(&mut self, token: &EditableTokenSpan) {
        self.selection = None;
        self.textarea.cancel_selection();
        self.textarea
            .move_cursor(CursorMove::Jump(token.row as u16, token.start_col as u16));
    }

    pub(super) fn adjacent_bar(&self, direction: i32, row: usize, col: usize) -> Option<BarSpan> {
        let bars = self.bar_spans();
        if direction < 0 {
            previous_bar_before_position(&bars, row, col)
        } else {
            next_bar_after_position(&bars, row, col)
        }
    }

    pub(super) fn focus_bar_cursor(&mut self, bar: &BarSpan) {
        self.selection = None;
        self.textarea.cancel_selection();
        let col = self.bar_cursor_col(bar);
        self.textarea
            .move_cursor(CursorMove::Jump(bar.row as u16, col as u16));
    }

    pub(super) fn selected_line_range(&self) -> (usize, usize) {
        match &self.selection {
            Some(StudioSelection::LineRange { anchor_row }) => {
                let row = self.textarea.cursor().0;
                ((*anchor_row).min(row), (*anchor_row).max(row))
            }
            Some(StudioSelection::EditableToken { row, .. }) => (*row, *row),
            Some(StudioSelection::EditableTokenRange { anchor, focus }) => {
                (anchor.row.min(focus.row), anchor.row.max(focus.row))
            }
            Some(StudioSelection::Bar { span }) => (span.row, span.row),
            Some(StudioSelection::BarRange { anchor, focus }) => {
                (anchor.row.min(focus.row), anchor.row.max(focus.row))
            }
            None => {
                let row = self.textarea.cursor().0;
                (row, row)
            }
        }
    }

    pub(super) fn cursor_label(&self) -> String {
        let cursor = self.textarea.cursor();
        format!("line {}, col {}", cursor.0 + 1, cursor.1 + 1)
    }

    pub(super) fn selection_label(&self) -> String {
        match &self.selection {
            Some(StudioSelection::EditableToken {
                row,
                start_col,
                token,
                ..
            }) => format!("token {} at line {}, col {}", token, row + 1, start_col + 1),
            Some(StudioSelection::EditableTokenRange { .. }) => {
                let selected = self.selected_editable_token_spans();
                match (selected.first(), selected.last()) {
                    (Some(first), Some(last)) if selected.len() == 1 => {
                        format!(
                            "token {} at line {}, col {}",
                            first.token,
                            first.row + 1,
                            first.start_col + 1
                        )
                    }
                    (Some(first), Some(last)) => format!(
                        "{} tokens from {} to {}",
                        selected.len(),
                        first.token,
                        last.token
                    ),
                    _ => "no tokens".to_string(),
                }
            }
            Some(StudioSelection::Bar { span }) => {
                format!("bar {} on line {}", span.index + 1, span.row + 1)
            }
            Some(StudioSelection::BarRange { anchor, focus }) => {
                let ((start_row, end_row), (start_index, end_index)) =
                    self.selected_bar_rectangle_bounds(anchor, focus);
                if start_row == end_row && start_index == end_index {
                    format!("bar {} on line {}", start_index + 1, start_row + 1)
                } else if start_row == end_row {
                    format!(
                        "bars {}..{} on line {}",
                        start_index + 1,
                        end_index + 1,
                        start_row + 1
                    )
                } else if start_index == end_index {
                    format!(
                        "bar {} across lines {}..{}",
                        start_index + 1,
                        start_row + 1,
                        end_row + 1
                    )
                } else {
                    format!(
                        "bars {}..{} across lines {}..{}",
                        start_index + 1,
                        end_index + 1,
                        start_row + 1,
                        end_row + 1
                    )
                }
            }
            Some(StudioSelection::LineRange { .. }) | None => {
                let (start, end) = self.selected_line_range();
                if start == end {
                    format!("line {}", start + 1)
                } else {
                    format!("lines {}..{}", start + 1, end + 1)
                }
            }
        }
    }

    pub(super) fn focus_editable_token(&self) -> Option<EditableTokenSpan> {
        match &self.selection {
            Some(StudioSelection::EditableToken {
                row,
                start_col,
                end_col,
                token,
                kind,
            }) => Some(EditableTokenSpan {
                row: *row,
                start_col: *start_col,
                end_col: *end_col,
                token: token.clone(),
                kind: *kind,
            }),
            Some(StudioSelection::EditableTokenRange { focus, .. }) => Some(focus.clone()),
            _ => None,
        }
    }

    pub(super) fn focus_bar(&self) -> Option<BarSpan> {
        match &self.selection {
            Some(StudioSelection::Bar { span }) => Some(span.clone()),
            Some(StudioSelection::BarRange { focus, .. }) => Some(focus.clone()),
            _ => None,
        }
    }

    pub(super) fn selected_editable_token_indices(&self) -> Vec<usize> {
        let notes = self.editable_token_spans();
        match &self.selection {
            Some(StudioSelection::EditableToken {
                row,
                start_col,
                end_col,
                token,
                kind,
            }) => {
                let selected = EditableTokenSpan {
                    row: *row,
                    start_col: *start_col,
                    end_col: *end_col,
                    token: token.clone(),
                    kind: *kind,
                };
                notes
                    .iter()
                    .position(|note| note == &selected)
                    .map(|index| vec![index])
                    .unwrap_or_default()
            }
            Some(StudioSelection::EditableTokenRange { anchor, focus }) => {
                let Some(anchor_index) = notes.iter().position(|note| note == anchor) else {
                    return Vec::new();
                };
                let Some(focus_index) = notes.iter().position(|note| note == focus) else {
                    return Vec::new();
                };
                let start = anchor_index.min(focus_index);
                let end = anchor_index.max(focus_index);
                (start..=end).collect()
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn selected_editable_token_spans(&self) -> Vec<EditableTokenSpan> {
        let notes = self.editable_token_spans();
        self.selected_editable_token_indices()
            .into_iter()
            .filter_map(|index| notes.get(index).cloned())
            .collect()
    }

    pub(super) fn selected_bar_spans(&self) -> Vec<BarSpan> {
        match &self.selection {
            Some(StudioSelection::Bar { span }) => self
                .bar_spans_on_line(span.row)
                .into_iter()
                .find(|bar| bar.index == span.index)
                .map(|bar| vec![bar])
                .unwrap_or_default(),
            Some(StudioSelection::BarRange { anchor, focus }) => {
                let ((start_row, end_row), (start_index, end_index)) =
                    self.selected_bar_rectangle_bounds(anchor, focus);
                (start_row..=end_row)
                    .flat_map(|row| {
                        self.bar_spans_on_line(row)
                            .into_iter()
                            .filter(move |bar| (start_index..=end_index).contains(&bar.index))
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn selected_bar_rectangle_bounds(
        &self,
        anchor: &BarSpan,
        focus: &BarSpan,
    ) -> ((usize, usize), (usize, usize)) {
        (
            (anchor.row.min(focus.row), anchor.row.max(focus.row)),
            (anchor.index.min(focus.index), anchor.index.max(focus.index)),
        )
    }

    pub(super) fn restore_editable_token_selection_from_indices(
        &mut self,
        selected_indices: &[usize],
    ) {
        let notes = self.editable_token_spans();
        match selected_indices {
            [] => {
                self.selection = None;
            }
            [index] => {
                if let Some(note) = notes.get(*index) {
                    self.selection = Some(StudioSelection::EditableToken {
                        row: note.row,
                        start_col: note.start_col,
                        end_col: note.end_col,
                        token: note.token.clone(),
                        kind: note.kind,
                    });
                }
            }
            indices => {
                let Some(first) = indices.first().and_then(|index| notes.get(*index)) else {
                    self.selection = None;
                    return;
                };
                let Some(last) = indices.last().and_then(|index| notes.get(*index)) else {
                    self.selection = None;
                    return;
                };
                self.selection = Some(StudioSelection::EditableTokenRange {
                    anchor: first.clone(),
                    focus: last.clone(),
                });
            }
        }
    }

    pub(super) fn restore_editable_token_selection_from_positions(
        &mut self,
        positions: &[(usize, usize)],
    ) {
        let notes = self.editable_token_spans();
        let resolved: Vec<EditableTokenSpan> = positions
            .iter()
            .filter_map(|(row, start_col)| {
                notes
                    .iter()
                    .find(|note| note.row == *row && note.start_col == *start_col)
                    .cloned()
            })
            .collect();
        match resolved.as_slice() {
            [] => {
                self.selection = None;
            }
            [token] => {
                self.selection = Some(StudioSelection::EditableToken {
                    row: token.row,
                    start_col: token.start_col,
                    end_col: token.end_col,
                    token: token.token.clone(),
                    kind: token.kind,
                });
            }
            [first, .., last] => {
                self.selection = Some(StudioSelection::EditableTokenRange {
                    anchor: first.clone(),
                    focus: last.clone(),
                });
            }
        }
    }

    pub(super) fn restore_bar_selection_from_positions(&mut self, positions: &[(usize, usize)]) {
        match positions {
            [] => {
                self.selection = None;
            }
            [(row, index)] => {
                let bars = self.bar_spans_on_line(*row);
                if let Some(bar) = bars.iter().find(|bar| bar.index == *index) {
                    self.selection = Some(StudioSelection::Bar { span: bar.clone() });
                } else {
                    self.selection = None;
                }
            }
            positions => {
                let Some((first_row, first_index)) = positions.first() else {
                    self.selection = None;
                    return;
                };
                let Some((last_row, last_index)) = positions.last() else {
                    self.selection = None;
                    return;
                };
                let first_bars = self.bar_spans_on_line(*first_row);
                let last_bars = self.bar_spans_on_line(*last_row);
                let Some(first) = first_bars.iter().find(|bar| bar.index == *first_index) else {
                    self.selection = None;
                    return;
                };
                let Some(last) = last_bars.iter().find(|bar| bar.index == *last_index) else {
                    self.selection = None;
                    return;
                };
                self.selection = Some(StudioSelection::BarRange {
                    anchor: first.clone(),
                    focus: last.clone(),
                });
            }
        }
    }

    pub(super) fn editable_token_at_or_after_cursor(
        &self,
        row: usize,
        col: usize,
    ) -> Option<EditableTokenSpan> {
        editable_token_at_or_near_col(self.editable_token_spans_on_line(row), col)
    }

    pub(super) fn bar_at_or_after_cursor(&self, row: usize, col: usize) -> Option<BarSpan> {
        bar_at_or_near_col(self.bar_spans_on_line(row), col)
    }

    pub(super) fn nearest_editable_token_on_line(
        &self,
        row: usize,
        col: usize,
    ) -> Option<EditableTokenSpan> {
        self.editable_token_spans_on_line(row)
            .into_iter()
            .min_by_key(|note| note.start_col.abs_diff(col))
    }

    pub(super) fn nearest_bar_on_line(&self, row: usize, col: usize) -> Option<BarSpan> {
        self.bar_spans_on_line(row)
            .into_iter()
            .min_by_key(|bar| bar.start_col.abs_diff(col))
    }

    pub(super) fn bar_on_line_by_index(&self, row: usize, index: usize) -> Option<BarSpan> {
        self.bar_spans_on_line(row)
            .into_iter()
            .find(|bar| bar.index == index)
    }

    pub(super) fn editable_token_spans(&self) -> Vec<EditableTokenSpan> {
        self.textarea
            .lines()
            .iter()
            .enumerate()
            .flat_map(|(row, _)| self.editable_token_spans_on_line(row))
            .collect()
    }

    pub(super) fn bar_spans(&self) -> Vec<BarSpan> {
        self.textarea
            .lines()
            .iter()
            .enumerate()
            .flat_map(|(row, _)| self.bar_spans_on_line(row))
            .collect()
    }

    pub(super) fn editable_token_spans_on_line(&self, row: usize) -> Vec<EditableTokenSpan> {
        self.textarea
            .lines()
            .get(row)
            .map(|line| editable_token_spans_in_line(row, line))
            .unwrap_or_default()
    }

    pub(super) fn move_cursor_to_adjacent_editable_token(&mut self, direction: i32) -> bool {
        let cursor = self.textarea.cursor();
        let next = self.adjacent_editable_token(direction, cursor.0, cursor.1);
        let Some(next) = next else {
            self.status_message = "No more editable tokens".into();
            return false;
        };

        self.focus_editable_token_cursor(&next);
        self.status_message = format!("Normal mode: {}", self.cursor_label());
        true
    }

    pub(super) fn move_cursor_to_adjacent_bar(&mut self, direction: i32) -> bool {
        let cursor = self.textarea.cursor();
        let next = self.adjacent_bar(direction, cursor.0, cursor.1);
        let Some(next) = next else {
            self.status_message = "No more bars".into();
            return false;
        };

        self.focus_bar_cursor(&next);
        self.status_message = format!("Normal mode: {}", self.cursor_label());
        true
    }

    pub(super) fn bar_spans_on_line(&self, row: usize) -> Vec<BarSpan> {
        self.textarea
            .lines()
            .get(row)
            .map(|line| bar_spans_in_line(row, line))
            .unwrap_or_default()
    }

    fn bar_cursor_col(&self, bar: &BarSpan) -> usize {
        let Some(line) = self.textarea.lines().get(bar.row) else {
            return bar.start_col.saturating_add(1);
        };
        bar_cursor_col_in_line(line, bar)
    }
}

fn bar_cursor_col_in_line(line: &str, bar: &BarSpan) -> usize {
    let chars: Vec<char> = line.chars().collect();
    ((bar.start_col + 1)..bar.end_col.saturating_sub(1))
        .find(|&col| chars.get(col).is_some_and(|ch| !ch.is_whitespace()))
        .unwrap_or_else(|| bar.start_col.saturating_add(1))
}

fn next_bar_after_position(bars: &[BarSpan], row: usize, col: usize) -> Option<BarSpan> {
    if let Some(bar) = bars
        .iter()
        .find(|bar| bar.row == row && col == bar.start_col)
        .cloned()
    {
        return Some(bar);
    }

    let current_index = bars
        .iter()
        .position(|bar| bar.row == row && col >= bar.start_col && col < bar.end_col);
    if let Some(index) = current_index {
        return bars.get(index + 1).cloned();
    }

    bars.iter()
        .find(|bar| bar.row > row || (bar.row == row && bar.start_col > col))
        .cloned()
}

fn previous_bar_before_position(bars: &[BarSpan], row: usize, col: usize) -> Option<BarSpan> {
    if let Some(bar) = bars
        .iter()
        .rev()
        .find(|bar| bar.row == row && col == bar.end_col.saturating_sub(1))
        .cloned()
    {
        return Some(bar);
    }

    let current_index = bars
        .iter()
        .position(|bar| bar.row == row && col >= bar.start_col && col < bar.end_col);
    if let Some(index) = current_index {
        return index
            .checked_sub(1)
            .and_then(|prev| bars.get(prev).cloned());
    }

    bars.iter()
        .rev()
        .find(|bar| bar.row < row || (bar.row == row && bar.start_col < col))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::{bar_cursor_col_in_line, next_bar_after_position, previous_bar_before_position};
    use crate::interface::studio::selection::bar_spans_in_line;

    #[test]
    fn next_bar_on_shared_pipe_prefers_right_bar() {
        let bars = bar_spans_in_line(0, "seq | C4 . | D4 . |");
        let next = next_bar_after_position(&bars, 0, 11).unwrap();
        assert_eq!(next.index, 1);
    }

    #[test]
    fn previous_bar_on_shared_pipe_prefers_left_bar() {
        let bars = bar_spans_in_line(0, "seq | C4 . | D4 . |");
        let previous = previous_bar_before_position(&bars, 0, 11).unwrap();
        assert_eq!(previous.index, 0);
    }

    #[test]
    fn bar_cursor_col_prefers_first_non_whitespace_in_bar() {
        let line = "seq | C4 . | D4 . |";
        let bars = bar_spans_in_line(0, line);
        assert_eq!(bar_cursor_col_in_line(line, &bars[0]), 6);
        assert_eq!(bar_cursor_col_in_line(line, &bars[1]), 13);
    }

    #[test]
    fn bar_cursor_col_falls_back_inside_empty_bar() {
        let line = "seq |    |";
        let bar = bar_spans_in_line(0, line).remove(0);
        assert_eq!(bar_cursor_col_in_line(line, &bar), bar.start_col + 1);
    }
}
