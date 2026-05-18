use super::input::PendingInput;
use super::settings::{format_track_header, parse_track_header};
use super::StudioApp;
use crossterm::event::{KeyCode, KeyEvent};
use miette::Result;
use ratatui_textarea::CursorMove;

impl StudioApp {
    pub(super) fn handle_goto_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.status_message = PendingInput::Goto.cancel_message().into();
            }
            KeyCode::Char('t') => {
                self.goto_adjacent_track(1);
            }
            KeyCode::Char('T') => {
                self.goto_adjacent_track(-1);
            }
            _ => {
                self.status_message = PendingInput::Goto.unknown_message();
            }
        }
        Ok(())
    }

    pub(super) fn handle_delete_structure_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.status_message = PendingInput::DeleteStructure.cancel_message().into();
            }
            KeyCode::Char('t') => {
                self.delete_current_track()?;
            }
            _ => {
                self.status_message = PendingInput::DeleteStructure.unknown_message();
            }
        }
        Ok(())
    }

    pub(super) fn toggle_current_track_mute(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(track_index) = current_track_index(&lines, cursor.0) else {
            self.status_message = "No track header found".into();
            return Ok(());
        };
        let header_rows = track_header_rows(&lines);
        let header_row = header_rows[track_index];
        let Some(header) = lines
            .get(header_row)
            .and_then(|line| parse_track_header(line))
        else {
            self.status_message = "Current track header is invalid".into();
            return Ok(());
        };

        let muted = !header.muted;
        lines[header_row] = format_track_header(&super::settings::TrackHeader { muted, ..header });
        self.apply_cursor_source_update(
            lines,
            (header_row, track_header_cursor_col()),
            if muted {
                "Track mute: on".into()
            } else {
                "Track mute: off".into()
            },
            None,
        )
    }

    pub(super) fn toggle_current_track_solo(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(track_index) = current_track_index(&lines, cursor.0) else {
            self.status_message = "No track header found".into();
            return Ok(());
        };
        let header_rows = track_header_rows(&lines);
        let header_row = header_rows[track_index];
        let Some(header) = lines
            .get(header_row)
            .and_then(|line| parse_track_header(line))
        else {
            self.status_message = "Current track header is invalid".into();
            return Ok(());
        };

        let solo = !header.solo;
        lines[header_row] = format_track_header(&super::settings::TrackHeader { solo, ..header });
        self.apply_cursor_source_update(
            lines,
            (header_row, track_header_cursor_col()),
            if solo {
                "Track solo: on".into()
            } else {
                "Track solo: off".into()
            },
            None,
        )
    }

    pub(super) fn clear_current_track_flags(&mut self) -> Result<()> {
        let mut lines = self.textarea.lines().to_vec();
        let header_rows = track_header_rows(&lines);
        let Some(&first_header_row) = header_rows.first() else {
            self.status_message = "No track header found".into();
            return Ok(());
        };

        for &header_row in &header_rows {
            let Some(header) = lines
                .get(header_row)
                .and_then(|line| parse_track_header(line))
            else {
                self.status_message = "Current track header is invalid".into();
                return Ok(());
            };

            lines[header_row] = format_track_header(&super::settings::TrackHeader {
                solo: false,
                muted: false,
                ..header
            });
        }
        self.apply_cursor_source_update(
            lines,
            (first_header_row, track_header_cursor_col()),
            "All track flags cleared".into(),
            None,
        )
    }

    fn goto_adjacent_track(&mut self, direction: i32) {
        let lines = self.textarea.lines().to_vec();
        let header_rows = track_header_rows(&lines);
        let Some(target_row) =
            adjacent_track_header_row(&header_rows, self.textarea.cursor().0, direction)
        else {
            self.status_message = if direction < 0 {
                "No previous track".into()
            } else {
                "No next track".into()
            };
            return;
        };

        self.selection = None;
        self.textarea.cancel_selection();
        self.textarea.move_cursor(CursorMove::Jump(
            target_row as u16,
            track_header_cursor_col() as u16,
        ));
        self.status_message = format!("Normal mode: {}", self.cursor_label());
    }

    fn delete_current_track(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(track_index) = current_track_index(&lines, cursor.0) else {
            self.status_message = "No track header found".into();
            return Ok(());
        };
        let header_rows = track_header_rows(&lines);
        let (start_row, end_row, deleted_header_row) =
            track_delete_span(&lines, &header_rows, track_index);
        let track_count = header_rows.len();
        if track_count <= 1 {
            self.status_message = "Cannot delete the last track".into();
            return Ok(());
        }

        lines.drain(start_row..end_row);
        let next_cursor_row = start_row.min(lines.len().saturating_sub(1));
        let next_header_rows = track_header_rows(&lines);
        let fallback_row = next_header_rows
            .iter()
            .find(|&&row| row >= deleted_header_row.min(next_cursor_row))
            .copied()
            .or_else(|| next_header_rows.last().copied())
            .unwrap_or(0);

        self.apply_cursor_source_update(
            lines,
            (fallback_row, track_header_cursor_col()),
            "Deleted current track".into(),
            None,
        )
    }
}

fn track_header_rows(lines: &[String]) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(row, line)| parse_track_header(line).map(|_| row))
        .collect()
}

fn current_track_index(lines: &[String], cursor_row: usize) -> Option<usize> {
    let header_rows = track_header_rows(lines);
    header_rows
        .iter()
        .rposition(|&row| row <= cursor_row)
        .or_else(|| (!header_rows.is_empty()).then_some(0))
}

fn adjacent_track_header_row(
    header_rows: &[usize],
    cursor_row: usize,
    direction: i32,
) -> Option<usize> {
    if direction < 0 {
        let current_index = header_rows.iter().rposition(|&row| row <= cursor_row)?;
        current_index
            .checked_sub(1)
            .and_then(|index| header_rows.get(index).copied())
    } else {
        match header_rows.iter().rposition(|&row| row <= cursor_row) {
            Some(index) => header_rows.get(index + 1).copied(),
            None => header_rows.first().copied(),
        }
    }
}

fn track_delete_span(
    lines: &[String],
    header_rows: &[usize],
    track_index: usize,
) -> (usize, usize, usize) {
    let header_row = header_rows[track_index];
    let start_row = if header_row > 0 && lines[header_row - 1].trim().is_empty() {
        header_row - 1
    } else {
        header_row
    };
    let end_row = header_rows
        .get(track_index + 1)
        .copied()
        .unwrap_or(lines.len());
    (start_row, end_row, header_row)
}

fn track_header_cursor_col() -> usize {
    2
}

#[cfg(test)]
mod tests {
    use super::{
        adjacent_track_header_row, current_track_index, track_delete_span, track_header_rows,
    };
    use crate::interface::studio::settings::{format_track_header, parse_track_header};

    #[test]
    fn track_header_rows_collect_headers_only() {
        let lines = vec![
            "# Piano: 1".to_string(),
            "seq | C4 |".to_string(),
            "".to_string(),
            "# Drums: 10 x".to_string(),
        ];
        assert_eq!(track_header_rows(&lines), vec![0, 3]);
    }

    #[test]
    fn current_track_index_uses_previous_header() {
        let lines = vec![
            "# Piano: 1".to_string(),
            "seq | C4 |".to_string(),
            "".to_string(),
            "# Drums: 10".to_string(),
        ];
        assert_eq!(current_track_index(&lines, 1), Some(0));
        assert_eq!(current_track_index(&lines, 3), Some(1));
    }

    #[test]
    fn adjacent_track_header_row_moves_between_headers() {
        let rows = vec![1, 4, 8];
        assert_eq!(adjacent_track_header_row(&rows, 0, 1), Some(1));
        assert_eq!(adjacent_track_header_row(&rows, 1, 1), Some(4));
        assert_eq!(adjacent_track_header_row(&rows, 7, -1), Some(1));
        assert_eq!(adjacent_track_header_row(&rows, 1, -1), None);
    }

    #[test]
    fn track_delete_span_removes_leading_separator_blank() {
        let lines = vec![
            "# Piano: 1".to_string(),
            "seq | C4 |".to_string(),
            "".to_string(),
            "# Drums: 10".to_string(),
            "kick | ^ |".to_string(),
        ];
        let headers = track_header_rows(&lines);
        assert_eq!(track_delete_span(&lines, &headers, 1), (2, 5, 3));
    }

    #[test]
    fn track_header_toggle_helpers_keep_canonical_solo_mute_order() {
        let header = parse_track_header("# Bass: 2 x s").unwrap();
        assert_eq!(format_track_header(&header), "# Bass: 2 s x");
    }

    #[test]
    fn track_header_clear_helper_removes_solo_and_mute_flags() {
        let header = parse_track_header("# Bass: 2 s x").unwrap();
        assert_eq!(
            format_track_header(&crate::interface::studio::settings::TrackHeader {
                solo: false,
                muted: false,
                ..header
            }),
            "# Bass: 2"
        );
    }
}
