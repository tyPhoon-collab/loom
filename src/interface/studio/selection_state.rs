use super::selection::{
    bar_at_or_near_col, bar_spans_in_line, note_at_or_near_col, note_spans_in_line,
    ordered_bar_span_bounds, BarSpan, NoteTokenSpan, StudioSelection,
};
use super::StudioApp;

impl StudioApp {
    pub(super) fn selected_line_range(&self) -> (usize, usize) {
        match &self.selection {
            Some(StudioSelection::LineRange { anchor_row }) => {
                let row = self.textarea.cursor().0;
                ((*anchor_row).min(row), (*anchor_row).max(row))
            }
            Some(StudioSelection::Note { row, .. }) => (*row, *row),
            Some(StudioSelection::NoteRange { anchor, focus }) => {
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
            Some(StudioSelection::Note {
                row,
                start_col,
                token,
                ..
            }) => format!("note {} at line {}, col {}", token, row + 1, start_col + 1),
            Some(StudioSelection::NoteRange { .. }) => {
                let selected = self.selected_note_spans();
                match (selected.first(), selected.last()) {
                    (Some(first), Some(last)) if selected.len() == 1 => {
                        format!(
                            "note {} at line {}, col {}",
                            first.token,
                            first.row + 1,
                            first.start_col + 1
                        )
                    }
                    (Some(first), Some(last)) => format!(
                        "{} notes from {} to {}",
                        selected.len(),
                        first.token,
                        last.token
                    ),
                    _ => "no notes".to_string(),
                }
            }
            Some(StudioSelection::Bar { span }) => {
                format!("bar {} on line {}", span.index + 1, span.row + 1)
            }
            Some(StudioSelection::BarRange { anchor, focus }) => {
                let (start, end) = ordered_bar_span_bounds(anchor, focus);
                if start == end {
                    format!("bar {} on line {}", start.index + 1, start.row + 1)
                } else if start.row == end.row {
                    format!(
                        "bars {}..{} on line {}",
                        start.index + 1,
                        end.index + 1,
                        start.row + 1
                    )
                } else {
                    format!(
                        "bars line {}:{} to line {}:{}",
                        start.row + 1,
                        start.index + 1,
                        end.row + 1,
                        end.index + 1
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

    pub(super) fn focus_note(&self) -> Option<NoteTokenSpan> {
        match &self.selection {
            Some(StudioSelection::Note {
                row,
                start_col,
                end_col,
                token,
                kind,
            }) => Some(NoteTokenSpan {
                row: *row,
                start_col: *start_col,
                end_col: *end_col,
                token: token.clone(),
                kind: *kind,
            }),
            Some(StudioSelection::NoteRange { focus, .. }) => Some(focus.clone()),
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

    pub(super) fn selected_note_indices(&self) -> Vec<usize> {
        let notes = self.note_token_spans();
        match &self.selection {
            Some(StudioSelection::Note {
                row,
                start_col,
                end_col,
                token,
                kind,
            }) => {
                let selected = NoteTokenSpan {
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
            Some(StudioSelection::NoteRange { anchor, focus }) => {
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

    pub(super) fn selected_note_spans(&self) -> Vec<NoteTokenSpan> {
        let notes = self.note_token_spans();
        self.selected_note_indices()
            .into_iter()
            .filter_map(|index| notes.get(index).cloned())
            .collect()
    }

    pub(super) fn selected_bar_spans(&self) -> Vec<BarSpan> {
        let bars = self.bar_spans();
        match &self.selection {
            Some(StudioSelection::Bar { span }) => bars
                .iter()
                .position(|bar| bar == span)
                .and_then(|index| bars.get(index).cloned())
                .map(|bar| vec![bar])
                .unwrap_or_default(),
            Some(StudioSelection::BarRange { anchor, focus }) => {
                let Some(anchor_index) = bars.iter().position(|bar| bar == anchor) else {
                    return Vec::new();
                };
                let Some(focus_index) = bars.iter().position(|bar| bar == focus) else {
                    return Vec::new();
                };
                let start = anchor_index.min(focus_index);
                let end = anchor_index.max(focus_index);
                (start..=end)
                    .filter_map(|index| bars.get(index).cloned())
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn restore_note_selection_from_indices(&mut self, selected_indices: &[usize]) {
        let notes = self.note_token_spans();
        match selected_indices {
            [] => {
                self.selection = None;
            }
            [index] => {
                if let Some(note) = notes.get(*index) {
                    self.selection = Some(StudioSelection::Note {
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
                self.selection = Some(StudioSelection::NoteRange {
                    anchor: first.clone(),
                    focus: last.clone(),
                });
            }
        }
    }

    pub(super) fn restore_bar_selection_from_row_indices(
        &mut self,
        row: usize,
        selected_indices: &[usize],
    ) {
        let bars = self.bar_spans_on_line(row);
        match selected_indices {
            [] => {
                self.selection = None;
            }
            [index] => {
                if let Some(bar) = bars.iter().find(|bar| bar.index == *index) {
                    self.selection = Some(StudioSelection::Bar { span: bar.clone() });
                }
            }
            indices => {
                let Some(first) = indices
                    .first()
                    .and_then(|index| bars.iter().find(|bar| bar.index == *index))
                else {
                    self.selection = None;
                    return;
                };
                let Some(last) = indices
                    .last()
                    .and_then(|index| bars.iter().find(|bar| bar.index == *index))
                else {
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

    pub(super) fn note_at_or_after_cursor(&self, row: usize, col: usize) -> Option<NoteTokenSpan> {
        note_at_or_near_col(self.note_spans_on_line(row), col)
    }

    pub(super) fn bar_at_or_after_cursor(&self, row: usize, col: usize) -> Option<BarSpan> {
        bar_at_or_near_col(self.bar_spans_on_line(row), col)
    }

    pub(super) fn nearest_note_on_line(&self, row: usize, col: usize) -> Option<NoteTokenSpan> {
        self.note_spans_on_line(row)
            .into_iter()
            .min_by_key(|note| note.start_col.abs_diff(col))
    }

    pub(super) fn nearest_bar_on_line(&self, row: usize, col: usize) -> Option<BarSpan> {
        self.bar_spans_on_line(row)
            .into_iter()
            .min_by_key(|bar| bar.start_col.abs_diff(col))
    }

    pub(super) fn note_token_spans(&self) -> Vec<NoteTokenSpan> {
        self.textarea
            .lines()
            .iter()
            .enumerate()
            .flat_map(|(row, _)| self.note_spans_on_line(row))
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

    pub(super) fn note_spans_on_line(&self, row: usize) -> Vec<NoteTokenSpan> {
        self.textarea
            .lines()
            .get(row)
            .map(|line| note_spans_in_line(row, line))
            .unwrap_or_default()
    }

    pub(super) fn bar_spans_on_line(&self, row: usize) -> Vec<BarSpan> {
        self.textarea
            .lines()
            .get(row)
            .map(|line| bar_spans_in_line(row, line))
            .unwrap_or_default()
    }
}
