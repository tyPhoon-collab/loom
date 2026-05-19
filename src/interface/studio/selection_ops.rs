use super::selection::{
    ordered_bar_span_bounds, ordered_unit_span_bounds, BarSpan, StudioSelection, UnitSpan,
};
use super::template_ops::TemplateCallSpan;
use super::{StudioApp, StudioMode};
use ratatui_textarea::CursorMove;

impl StudioApp {
    pub(super) fn enter_line_select_mode(&mut self) {
        self.mode = StudioMode::Select;
        self.selection = Some(StudioSelection::LineRange {
            anchor_row: self.textarea.cursor().0,
        });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    pub(super) fn enter_note_select_mode(&mut self) {
        let cursor = self.textarea.cursor();
        if let Some(note) = self.unit_at_or_after_cursor(cursor.0, cursor.1) {
            self.mode = StudioMode::Select;
            self.selection = Some(StudioSelection::Unit {
                row: note.row,
                start_col: note.start_col,
                end_col: note.end_col,
                token: note.token,
                kind: note.kind,
            });
            self.sync_selection_visual();
            self.status_message = format!("Select mode: {}", self.selection_label());
            return;
        }
        let Some(call) = self.template_call_at_or_after_cursor(cursor.0, cursor.1) else {
            self.status_message = "No unit on this line".into();
            return;
        };

        self.mode = StudioMode::Select;
        self.selection = Some(StudioSelection::TemplateCall { span: call });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    pub(super) fn enter_bar_select_mode(&mut self) {
        let cursor = self.textarea.cursor();
        let Some(bar) = self.bar_at_or_after_cursor(cursor.0, cursor.1) else {
            self.status_message = "No bar on this line".into();
            return;
        };

        self.mode = StudioMode::Select;
        self.selection = Some(StudioSelection::Bar { span: bar });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    pub(super) fn enter_line_bar_select_mode(&mut self) {
        let row = self.textarea.cursor().0;
        let bars = self.bar_spans_on_line(row);
        match (bars.first(), bars.last()) {
            (Some(first), Some(last)) if first == last => {
                self.mode = StudioMode::Select;
                self.selection = Some(StudioSelection::Bar {
                    span: first.clone(),
                });
            }
            (Some(first), Some(last)) => {
                self.mode = StudioMode::Select;
                self.selection = Some(StudioSelection::BarRange {
                    anchor: first.clone(),
                    focus: last.clone(),
                });
            }
            _ => {
                self.status_message = "No bars on this line".into();
                return;
            }
        }
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    pub(super) fn exit_select_mode(&mut self) {
        self.selection = None;
        self.textarea.cancel_selection();
        self.mode = StudioMode::Normal;
        self.status_message = format!("Normal mode: {}", self.cursor_label());
    }

    pub(super) fn move_select_horizontal(&mut self, direction: i32) {
        match self.selection {
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. }) => {
                self.move_bar_selection(direction);
            }
            Some(
                StudioSelection::TemplateCall { .. } | StudioSelection::TemplateCallRange { .. },
            ) => {
                self.move_template_call_selection(direction);
            }
            Some(StudioSelection::Unit { .. } | StudioSelection::UnitRange { .. }) => {
                self.move_unit_selection(direction);
            }
            Some(StudioSelection::LineRange { .. }) => {
                self.status_message = "Line selection moves vertically only".into();
            }
            None => {
                self.status_message = "No selection".into();
            }
        }
    }

    pub(super) fn expand_select_horizontal(&mut self, direction: i32) {
        match self.selection {
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. }) => {
                self.expand_bar_selection(direction);
            }
            Some(
                StudioSelection::TemplateCall { .. } | StudioSelection::TemplateCallRange { .. },
            ) => {
                self.expand_template_call_selection(direction);
            }
            Some(StudioSelection::Unit { .. } | StudioSelection::UnitRange { .. }) => {
                self.expand_unit_selection(direction);
            }
            Some(StudioSelection::LineRange { .. }) => {
                self.status_message = "Line selection expands vertically only".into();
            }
            None => {
                self.status_message = "No selection".into();
            }
        }
    }

    pub(super) fn expand_select_vertical(&mut self, direction: i32) {
        match self.selection {
            Some(StudioSelection::LineRange { .. }) => {
                self.expand_line_selection_vertical(direction);
            }
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. }) => {
                self.expand_bar_selection_vertical(direction);
            }
            Some(
                StudioSelection::TemplateCall { .. } | StudioSelection::TemplateCallRange { .. },
            ) => {
                self.expand_template_call_selection_vertical(direction);
            }
            Some(StudioSelection::Unit { .. } | StudioSelection::UnitRange { .. }) => {
                self.expand_unit_selection_vertical(direction);
            }
            None => {
                self.status_message = "No selection".into();
            }
        }
    }

    pub(super) fn move_template_call_selection(&mut self, direction: i32) {
        let Some(current) = self.focus_template_call() else {
            self.status_message = "No template call selected. Press v first.".into();
            return;
        };
        let spans = self.template_call_spans();
        let Some(index) = spans.iter().position(|span| span == &current) else {
            self.status_message = "Selected template call no longer exists".into();
            return;
        };
        let next_index = if direction < 0 {
            index.checked_sub(1)
        } else {
            (index + 1 < spans.len()).then_some(index + 1)
        };
        let Some(next_index) = next_index else {
            self.status_message = "No more template calls".into();
            return;
        };
        self.set_template_call_selection(spans[next_index].clone());
    }

    pub(super) fn expand_template_call_selection(&mut self, direction: i32) {
        let Some(focus) = self.focus_template_call() else {
            self.status_message = "No template call selected. Press v first.".into();
            return;
        };
        let spans = self.template_call_spans();
        let Some(focus_index) = spans.iter().position(|span| span == &focus) else {
            self.status_message = "Selected template call no longer exists".into();
            return;
        };
        let next_index = if direction < 0 {
            focus_index.checked_sub(1)
        } else {
            (focus_index + 1 < spans.len()).then_some(focus_index + 1)
        };
        let Some(next_index) = next_index else {
            self.status_message = "No more template calls".into();
            return;
        };
        self.expand_template_call_selection_to(spans[next_index].clone());
    }

    pub(super) fn expand_template_call_selection_vertical(&mut self, direction: i32) {
        let Some(focus) = self.focus_template_call() else {
            self.status_message = "No template call selected. Press v first.".into();
            return;
        };
        let next_row = if direction < 0 {
            focus.row.checked_sub(1)
        } else {
            (focus.row + 1 < self.textarea.lines().len()).then_some(focus.row + 1)
        };
        let Some(next_row) = next_row else {
            self.status_message = "No more lines".into();
            return;
        };
        let Some(span) = self.nearest_template_call_on_line(next_row, focus.start_col) else {
            self.status_message = "No template call on target line".into();
            return;
        };
        self.expand_template_call_selection_to(span);
    }

    pub(super) fn expand_line_selection_vertical(&mut self, direction: i32) {
        match self.selection {
            Some(StudioSelection::LineRange { .. }) => {
                let cursor_move = if direction < 0 {
                    CursorMove::Up
                } else {
                    CursorMove::Down
                };
                self.textarea.move_cursor(cursor_move);
                self.sync_selection_visual();
                self.status_message = format!("Select mode: {}", self.selection_label());
            }
            _ => {
                self.status_message = "Current selection is not a line selection".into();
            }
        }
    }

    pub(super) fn move_unit_selection(&mut self, direction: i32) {
        let Some(current) = self.focus_unit() else {
            self.status_message = "No unit selected. Press v first.".into();
            return;
        };
        let notes = self.unit_spans();
        let Some(index) = notes.iter().position(|note| note == &current) else {
            self.status_message = "Selected unit no longer exists".into();
            return;
        };

        let next_index = if direction < 0 {
            index.checked_sub(1)
        } else {
            (index + 1 < notes.len()).then_some(index + 1)
        };

        let Some(next_index) = next_index else {
            self.status_message = "No more units".into();
            return;
        };

        self.set_unit_selection(notes[next_index].clone());
    }

    pub(super) fn expand_unit_selection(&mut self, direction: i32) {
        let Some(focus) = self.focus_unit() else {
            self.status_message = "No unit selected. Press v first.".into();
            return;
        };
        let notes = self.unit_spans();
        let Some(focus_index) = notes.iter().position(|note| note == &focus) else {
            self.status_message = "Selected unit no longer exists".into();
            return;
        };

        let next_index = if direction < 0 {
            focus_index.checked_sub(1)
        } else {
            (focus_index + 1 < notes.len()).then_some(focus_index + 1)
        };

        let Some(next_index) = next_index else {
            self.status_message = "No more units".into();
            return;
        };

        self.expand_unit_selection_to(notes[next_index].clone());
    }

    pub(super) fn expand_unit_selection_vertical(&mut self, direction: i32) {
        let Some(focus) = self.focus_unit() else {
            self.status_message = "No unit selected. Press v first.".into();
            return;
        };
        let next_row = if direction < 0 {
            focus.row.checked_sub(1)
        } else {
            (focus.row + 1 < self.textarea.lines().len()).then_some(focus.row + 1)
        };
        let Some(next_row) = next_row else {
            self.status_message = "No more lines".into();
            return;
        };
        let Some(note) = self.nearest_unit_on_line(next_row, focus.start_col) else {
            self.status_message = "No unit on target line".into();
            return;
        };
        self.expand_unit_selection_to(note);
    }

    pub(super) fn move_bar_selection(&mut self, direction: i32) {
        let Some(current) = self.focus_bar() else {
            self.status_message = "No bar selected. Press b first.".into();
            return;
        };
        let bars = self.bar_spans();
        let Some(index) = bars.iter().position(|bar| bar == &current) else {
            self.status_message = "Selected bar no longer exists".into();
            return;
        };

        let next_index = if direction < 0 {
            index.checked_sub(1)
        } else {
            (index + 1 < bars.len()).then_some(index + 1)
        };

        let Some(next_index) = next_index else {
            self.status_message = "No more bars".into();
            return;
        };

        self.set_bar_selection(bars[next_index].clone());
    }

    pub(super) fn expand_bar_selection(&mut self, direction: i32) {
        let Some(focus) = self.focus_bar() else {
            self.status_message = "No bar selected. Press b first.".into();
            return;
        };
        let bars = self.bar_spans();
        let Some(focus_index) = bars.iter().position(|bar| bar == &focus) else {
            self.status_message = "Selected bar no longer exists".into();
            return;
        };

        let next_index = if direction < 0 {
            focus_index.checked_sub(1)
        } else {
            (focus_index + 1 < bars.len()).then_some(focus_index + 1)
        };

        let Some(next_index) = next_index else {
            self.status_message = "No more bars".into();
            return;
        };

        self.expand_bar_selection_to(bars[next_index].clone());
    }

    pub(super) fn expand_bar_selection_vertical(&mut self, direction: i32) {
        let Some(focus) = self.focus_bar() else {
            self.status_message = "No bar selected. Press b first.".into();
            return;
        };
        let next_row = if direction < 0 {
            focus.row.checked_sub(1)
        } else {
            (focus.row + 1 < self.textarea.lines().len()).then_some(focus.row + 1)
        };
        let Some(next_row) = next_row else {
            self.status_message = "No more lines".into();
            return;
        };
        let Some(bar) = self
            .bar_on_line_by_index(next_row, focus.index)
            .or_else(|| self.nearest_bar_on_line(next_row, focus.start_col))
        else {
            self.status_message = "No matching bar on target line".into();
            return;
        };
        self.expand_bar_selection_to(bar);
    }

    pub(super) fn move_selection_vertical(&mut self, direction: i32) {
        match &self.selection {
            Some(StudioSelection::LineRange { .. }) => {
                let cursor_move = if direction < 0 {
                    CursorMove::Up
                } else {
                    CursorMove::Down
                };
                self.textarea.move_cursor(cursor_move);
                self.sync_selection_visual();
                self.status_message = format!("Select mode: {}", self.selection_label());
            }
            Some(StudioSelection::Bar {
                span:
                    BarSpan {
                        row,
                        start_col,
                        index,
                        ..
                    },
            })
            | Some(StudioSelection::BarRange {
                focus:
                    BarSpan {
                        row,
                        start_col,
                        index,
                        ..
                    },
                ..
            }) => {
                let next_row = if direction < 0 {
                    (*row).checked_sub(1)
                } else {
                    (*row + 1 < self.textarea.lines().len()).then_some(*row + 1)
                };
                let Some(next_row) = next_row else {
                    self.status_message = "No more lines".into();
                    return;
                };
                let Some(bar) = self
                    .bar_on_line_by_index(next_row, *index)
                    .or_else(|| self.nearest_bar_on_line(next_row, *start_col))
                else {
                    self.status_message = "No matching bar on target line".into();
                    return;
                };
                self.set_bar_selection(bar);
            }
            Some(StudioSelection::Unit { row, start_col, .. })
            | Some(StudioSelection::UnitRange {
                focus: UnitSpan { row, start_col, .. },
                ..
            }) => {
                let next_row = if direction < 0 {
                    (*row).checked_sub(1)
                } else {
                    (*row + 1 < self.textarea.lines().len()).then_some(*row + 1)
                };
                let Some(next_row) = next_row else {
                    self.status_message = "No more lines".into();
                    return;
                };
                let Some(note) = self.nearest_unit_on_line(next_row, *start_col) else {
                    self.status_message = "No unit on target line".into();
                    return;
                };
                self.set_unit_selection(note);
            }
            Some(StudioSelection::TemplateCall { span })
            | Some(StudioSelection::TemplateCallRange { focus: span, .. }) => {
                let next_row = if direction < 0 {
                    span.row.checked_sub(1)
                } else {
                    (span.row + 1 < self.textarea.lines().len()).then_some(span.row + 1)
                };
                let Some(next_row) = next_row else {
                    self.status_message = "No more lines".into();
                    return;
                };
                let Some(next_span) = self.nearest_template_call_on_line(next_row, span.start_col)
                else {
                    self.status_message = "No template call on target line".into();
                    return;
                };
                self.set_template_call_selection(next_span);
            }
            None => {
                self.status_message = "No selection".into();
            }
        }
    }

    pub(super) fn set_unit_selection(&mut self, note: UnitSpan) {
        self.selection = Some(StudioSelection::Unit {
            row: note.row,
            start_col: note.start_col,
            end_col: note.end_col,
            token: note.token,
            kind: note.kind,
        });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    pub(super) fn set_bar_selection(&mut self, bar: BarSpan) {
        self.selection = Some(StudioSelection::Bar { span: bar });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    pub(super) fn set_template_call_selection(&mut self, span: TemplateCallSpan) {
        self.selection = Some(StudioSelection::TemplateCall { span });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    pub(super) fn expand_unit_selection_to(&mut self, focus: UnitSpan) {
        let anchor = match &self.selection {
            Some(StudioSelection::Unit {
                row,
                start_col,
                end_col,
                token,
                kind,
            }) => UnitSpan {
                row: *row,
                start_col: *start_col,
                end_col: *end_col,
                token: token.clone(),
                kind: *kind,
            },
            Some(StudioSelection::UnitRange { anchor, .. }) => anchor.clone(),
            _ => {
                self.status_message = "Current selection is not a unit selection".into();
                return;
            }
        };

        self.selection = Some(StudioSelection::UnitRange { anchor, focus });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    pub(super) fn expand_bar_selection_to(&mut self, focus: BarSpan) {
        let anchor = match &self.selection {
            Some(StudioSelection::Bar { span }) => span.clone(),
            Some(StudioSelection::BarRange { anchor, .. }) => anchor.clone(),
            _ => {
                self.status_message = "Current selection is not a bar selection".into();
                return;
            }
        };

        self.selection = Some(StudioSelection::BarRange { anchor, focus });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    pub(super) fn expand_template_call_selection_to(&mut self, focus: TemplateCallSpan) {
        let anchor = match &self.selection {
            Some(StudioSelection::TemplateCall { span }) => span.clone(),
            Some(StudioSelection::TemplateCallRange { anchor, .. }) => anchor.clone(),
            _ => {
                self.status_message = "Current selection is not a template call selection".into();
                return;
            }
        };

        self.selection = Some(StudioSelection::TemplateCallRange { anchor, focus });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    pub(super) fn sync_selection_visual(&mut self) {
        let Some(selection) = self.selection.clone() else {
            return;
        };

        self.textarea.cancel_selection();
        match selection {
            StudioSelection::Unit {
                row,
                start_col,
                end_col,
                ..
            } => {
                self.textarea
                    .move_cursor(CursorMove::Jump(row as u16, start_col as u16));
                self.textarea.start_selection();
                self.textarea
                    .move_cursor(CursorMove::Jump(row as u16, end_col as u16));
            }
            StudioSelection::UnitRange { anchor, focus } => {
                let (start, end) = ordered_unit_span_bounds(&anchor, &focus);
                self.textarea
                    .move_cursor(CursorMove::Jump(start.row as u16, start.start_col as u16));
                self.textarea.start_selection();
                self.textarea
                    .move_cursor(CursorMove::Jump(end.row as u16, end.end_col as u16));
            }
            StudioSelection::Bar { span } => {
                self.textarea
                    .move_cursor(CursorMove::Jump(span.row as u16, span.start_col as u16));
                self.textarea.start_selection();
                self.textarea
                    .move_cursor(CursorMove::Jump(span.row as u16, span.end_col as u16));
            }
            StudioSelection::BarRange { anchor, focus } => {
                if anchor.row == focus.row {
                    let (start, end) = ordered_bar_span_bounds(&anchor, &focus);
                    self.textarea
                        .move_cursor(CursorMove::Jump(start.row as u16, start.start_col as u16));
                    self.textarea.start_selection();
                    self.textarea
                        .move_cursor(CursorMove::Jump(end.row as u16, end.end_col as u16));
                } else {
                    self.textarea
                        .move_cursor(CursorMove::Jump(focus.row as u16, focus.start_col as u16));
                }
            }
            StudioSelection::TemplateCall { span } => {
                self.textarea
                    .move_cursor(CursorMove::Jump(span.row as u16, span.start_col as u16));
                self.textarea.start_selection();
                self.textarea
                    .move_cursor(CursorMove::Jump(span.row as u16, span.end_col as u16));
            }
            StudioSelection::TemplateCallRange { anchor, focus } => {
                let (start, end) = if (anchor.row, anchor.start_col) <= (focus.row, focus.start_col)
                {
                    (anchor, focus)
                } else {
                    (focus, anchor)
                };
                self.textarea
                    .move_cursor(CursorMove::Jump(start.row as u16, start.start_col as u16));
                self.textarea.start_selection();
                self.textarea
                    .move_cursor(CursorMove::Jump(end.row as u16, end.end_col as u16));
            }
            StudioSelection::LineRange { anchor_row } => {
                let current_row = self.textarea.cursor().0;
                let anchor_col = self.line_len(anchor_row);
                let current_col = self.line_len(current_row);

                if current_row >= anchor_row {
                    self.textarea
                        .move_cursor(CursorMove::Jump(anchor_row as u16, 0));
                    self.textarea.start_selection();
                    self.textarea
                        .move_cursor(CursorMove::Jump(current_row as u16, current_col as u16));
                } else {
                    self.textarea
                        .move_cursor(CursorMove::Jump(anchor_row as u16, anchor_col as u16));
                    self.textarea.start_selection();
                    self.textarea
                        .move_cursor(CursorMove::Jump(current_row as u16, 0));
                }
            }
        }
    }

    pub(super) fn line_len(&self, row: usize) -> usize {
        self.textarea
            .lines()
            .get(row)
            .map(|line| line.chars().count())
            .unwrap_or(0)
    }
}
