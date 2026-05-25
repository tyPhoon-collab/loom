use super::input::PendingInput;
use super::selection::{
    bar_at_or_near_col, bar_spans_in_line, is_seq_line, lane_head_token, replace_char_range,
};
use super::settings::{format_track_header, parse_track_header};
use super::{lookup_key_action, KeyBinding, KeySpec, StudioApp};
use crossterm::event::{KeyCode, KeyEvent};
use miette::Result;
use ratatui_textarea::CursorMove;

#[derive(Clone, Copy, Debug)]
enum GotoKeyAction {
    Cancel,
    NextTrack,
    PreviousTrack,
    GotoTemplateDefinition,
}

#[derive(Clone, Copy, Debug)]
enum DeleteStructureKeyAction {
    Cancel,
    DeleteSeqLine,
    DeleteNoteHeadLine,
    DeleteTrack,
    DeleteSeparator,
    DeleteTemplateDefinition,
    DeleteBar,
    DeleteVelocityModifier,
    DeletePitchModifier,
    BeginDeleteTemplateMacro,
}

const GOTO_KEY_BINDINGS: &[KeyBinding<GotoKeyAction>] = &[
    KeyBinding {
        spec: KeySpec::Code(KeyCode::Esc),
        action: GotoKeyAction::Cancel,
    },
    KeyBinding {
        spec: KeySpec::PlainChar('t'),
        action: GotoKeyAction::NextTrack,
    },
    KeyBinding {
        spec: KeySpec::ShiftChar('t'),
        action: GotoKeyAction::PreviousTrack,
    },
    KeyBinding {
        spec: KeySpec::PlainChar('d'),
        action: GotoKeyAction::GotoTemplateDefinition,
    },
];

const DELETE_STRUCTURE_KEY_BINDINGS: &[KeyBinding<DeleteStructureKeyAction>] = &[
    KeyBinding {
        spec: KeySpec::Code(KeyCode::Esc),
        action: DeleteStructureKeyAction::Cancel,
    },
    KeyBinding {
        spec: KeySpec::PlainChar('s'),
        action: DeleteStructureKeyAction::DeleteSeqLine,
    },
    KeyBinding {
        spec: KeySpec::PlainChar('l'),
        action: DeleteStructureKeyAction::DeleteNoteHeadLine,
    },
    KeyBinding {
        spec: KeySpec::PlainChar('t'),
        action: DeleteStructureKeyAction::DeleteTrack,
    },
    KeyBinding {
        spec: KeySpec::PlainChar('h'),
        action: DeleteStructureKeyAction::DeleteSeparator,
    },
    KeyBinding {
        spec: KeySpec::ShiftChar('t'),
        action: DeleteStructureKeyAction::DeleteTemplateDefinition,
    },
    KeyBinding {
        spec: KeySpec::PlainChar('b'),
        action: DeleteStructureKeyAction::DeleteBar,
    },
    KeyBinding {
        spec: KeySpec::PlainChar('v'),
        action: DeleteStructureKeyAction::DeleteVelocityModifier,
    },
    KeyBinding {
        spec: KeySpec::PlainChar('p'),
        action: DeleteStructureKeyAction::DeletePitchModifier,
    },
    KeyBinding {
        spec: KeySpec::PlainChar('m'),
        action: DeleteStructureKeyAction::BeginDeleteTemplateMacro,
    },
];

impl StudioApp {
    pub(super) fn handle_goto_key(&mut self, key: KeyEvent) -> Result<()> {
        let Some(action) = lookup_key_action(GOTO_KEY_BINDINGS, &key) else {
            self.status_message = PendingInput::Goto.unknown_message();
            return Ok(());
        };

        match action {
            GotoKeyAction::Cancel => {
                self.status_message = PendingInput::Goto.cancel_message().into();
            }
            GotoKeyAction::NextTrack => self.goto_adjacent_track(1),
            GotoKeyAction::PreviousTrack => self.goto_adjacent_track(-1),
            GotoKeyAction::GotoTemplateDefinition => self.goto_current_template_definition()?,
        }
        Ok(())
    }

    pub(super) fn handle_delete_structure_key(&mut self, key: KeyEvent) -> Result<()> {
        let Some(action) = lookup_key_action(DELETE_STRUCTURE_KEY_BINDINGS, &key) else {
            self.status_message = PendingInput::DeleteStructure.unknown_message();
            return Ok(());
        };

        match action {
            DeleteStructureKeyAction::Cancel => {
                self.status_message = PendingInput::DeleteStructure.cancel_message().into();
            }
            DeleteStructureKeyAction::DeleteSeqLine => self.delete_current_seq_line()?,
            DeleteStructureKeyAction::DeleteNoteHeadLine => self.delete_current_note_head_line()?,
            DeleteStructureKeyAction::DeleteTrack => self.delete_current_track()?,
            DeleteStructureKeyAction::DeleteSeparator => self.delete_current_separator()?,
            DeleteStructureKeyAction::DeleteTemplateDefinition => {
                self.delete_current_template_definition()?
            }
            DeleteStructureKeyAction::DeleteBar => self.delete_current_bar()?,
            DeleteStructureKeyAction::DeleteVelocityModifier => {
                self.delete_current_modifier_line("v")?
            }
            DeleteStructureKeyAction::DeletePitchModifier => {
                self.delete_current_modifier_line("p")?
            }
            DeleteStructureKeyAction::BeginDeleteTemplateMacro => {
                self.begin_delete_template_macro()?
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

    fn delete_current_seq_line(&mut self) -> Result<()> {
        self.delete_current_line_matching(
            is_seq_line,
            "Delete seq line needs the cursor on a seq line",
            "Deleted seq line",
        )
    }

    fn delete_current_note_head_line(&mut self) -> Result<()> {
        self.delete_current_line_matching(
            |line| lane_head_token(line).is_some() && !is_seq_line(line),
            "Delete note-head line needs the cursor on a note-head or drum lane line",
            "Deleted note-head line",
        )
    }

    fn delete_current_separator(&mut self) -> Result<()> {
        self.delete_current_line_matching(
            |line| line.trim() == "---",
            "Delete separator needs the cursor on a --- line",
            "Deleted separator",
        )
    }

    fn delete_current_modifier_line(&mut self, label: &str) -> Result<()> {
        let expected = match label {
            "v" => "velocity",
            "p" => "pitch",
            _ => "modifier",
        };
        self.delete_current_line_matching(
            |line| modifier_line_kind(line) == Some(label),
            &format!(
                "Delete {} modifier needs the cursor on a {} line",
                expected, label
            ),
            &format!("Deleted {} modifier line", expected),
        )
    }

    fn delete_current_bar(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(cursor.0) else {
            self.status_message = "No current line".into();
            return Ok(());
        };

        let bars = bar_spans_in_line(cursor.0, line);
        let Some(bar) = bar_at_or_near_col(bars.clone(), cursor.1) else {
            self.status_message = "Delete bar needs a line with bars".into();
            return Ok(());
        };
        if bars.len() <= 1 {
            self.status_message = "Cannot delete the last bar on a line".into();
            return Ok(());
        }

        replace_char_range(line, bar.start_col + 1, bar.end_col, "");
        self.apply_cursor_source_update(
            lines,
            (cursor.0, bar.start_col),
            "Deleted bar".into(),
            None,
        )
    }

    fn begin_delete_template_macro(&mut self) -> Result<()> {
        let Some(call) = self.current_template_call_at_cursor() else {
            self.status_message =
                "Template macro delete needs the cursor on a template call".into();
            return Ok(());
        };
        let row = call.row;
        let cursor_col = self.textarea.cursor().1;
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(row) else {
            self.status_message = "Selected template call no longer exists".into();
            return Ok(());
        };
        let Some((updated, deleted_macro)) =
            template_call_text_without_macro_at_cursor(cursor_col, &call)
        else {
            self.status_message =
                "Delete template macro needs the cursor on arp, rev, or strum".into();
            return Ok(());
        };
        replace_char_range(line, call.start_col, call.end_col, &updated);
        self.apply_cursor_source_update(
            lines,
            (row, call.start_col),
            format!(
                "Deleted {} macro from @{}",
                deleted_macro, call.template_name
            ),
            None,
        )
    }

    fn delete_current_template_definition(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let lines = self.textarea.lines().to_vec();
        let Some((start_row, end_row, header_row, template_name)) =
            template_definition_delete_span(&lines, cursor.0)
        else {
            self.status_message =
                "Delete template needs the cursor on a template definition block".into();
            return Ok(());
        };

        let mut updated = lines;
        updated.drain(start_row..end_row);
        let next_cursor_row = start_row.min(updated.len().saturating_sub(1));
        self.apply_cursor_source_update(
            updated,
            (
                next_cursor_row,
                if next_cursor_row == header_row { 2 } else { 0 },
            ),
            format!("Deleted template definition: @{}", template_name),
            None,
        )
    }

    fn delete_current_line_matching<F>(
        &mut self,
        predicate: F,
        invalid_message: &str,
        status_message: &str,
    ) -> Result<()>
    where
        F: Fn(&str) -> bool,
    {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get(cursor.0) else {
            self.status_message = "No current line".into();
            return Ok(());
        };
        if !predicate(line) {
            self.status_message = invalid_message.into();
            return Ok(());
        }

        lines.remove(cursor.0);
        let next_row = cursor.0.min(lines.len().saturating_sub(1));
        self.apply_cursor_source_update(lines, (next_row, 0), status_message.into(), None)
    }
}

fn modifier_line_kind(line: &str) -> Option<&'static str> {
    let pipe_col = line.chars().position(|ch| ch == '|')?;
    match line.chars().take(pipe_col).collect::<String>().trim() {
        "v" => Some("v"),
        "p" => Some("p"),
        _ => None,
    }
}

fn template_definition_delete_span(
    lines: &[String],
    cursor_row: usize,
) -> Option<(usize, usize, usize, String)> {
    let header_row = (0..=cursor_row).rev().find(|&row| {
        is_template_header(line_at(lines, row)) && cursor_row <= template_body_end(lines, row)
    })?;
    let header_line = lines.get(header_row)?;
    let template_name = header_line.trim().strip_prefix("# @")?.trim().to_string();
    let body_end = template_body_end(lines, header_row);
    let start_row = if header_row > 0 && lines[header_row - 1].trim().is_empty() {
        header_row - 1
    } else {
        header_row
    };
    Some((start_row, body_end + 1, header_row, template_name))
}

fn line_at(lines: &[String], row: usize) -> &str {
    lines.get(row).map(String::as_str).unwrap_or("")
}

fn is_template_header(line: &str) -> bool {
    line.trim().starts_with("# @")
}

fn template_body_end(lines: &[String], header_row: usize) -> usize {
    let mut row = header_row;
    while row + 1 < lines.len() {
        let next = row + 1;
        let next_line = line_at(lines, next);
        if next_line.trim().is_empty() || next_line.trim().starts_with('#') {
            break;
        }
        row = next;
    }
    row
}

fn template_call_text_without_macro_at_cursor(
    cursor_col: usize,
    call: &super::template_ops::TemplateCallSpan,
) -> Option<(String, &'static str)> {
    if cursor_col < call.start_col || cursor_col > call.end_col {
        return None;
    }

    let body = call.raw_text.strip_prefix("[@")?;
    let closing = body.find(']')?;
    let inside = &body[..closing];
    let suffix = &body[closing + 1..];

    let parts: Vec<&str> = inside.split_whitespace().collect();
    let name = *parts.first()?;
    let macro_index = parts
        .iter()
        .position(|part| matches!(*part, "arp" | "rev" | "strum"))?;
    let deleted_macro = parts[macro_index];

    let mut rewritten = String::from("[@");
    rewritten.push_str(name);
    for (index, part) in parts.iter().enumerate().skip(1) {
        if index == macro_index {
            continue;
        }
        rewritten.push(' ');
        rewritten.push_str(part);
    }
    rewritten.push(']');
    rewritten.push_str(suffix);

    Some((
        rewritten,
        match deleted_macro {
            "arp" => "arp",
            "rev" => "rev",
            "strum" => "strum",
            _ => unreachable!(),
        },
    ))
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
        adjacent_track_header_row, current_track_index, modifier_line_kind,
        template_call_text_without_macro_at_cursor, template_definition_delete_span,
        track_delete_span, track_header_rows,
    };
    use crate::interface::studio::settings::{format_track_header, parse_track_header};
    use crate::interface::studio::template_ops::template_call_spans_in_line;

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

    #[test]
    fn modifier_line_kind_detects_velocity_and_pitch_lines() {
        assert_eq!(modifier_line_kind("v  | . . . |"), Some("v"));
        assert_eq!(modifier_line_kind("p  | 0 1 2 |"), Some("p"));
        assert_eq!(modifier_line_kind("seq | C4 |"), None);
    }

    #[test]
    fn template_definition_delete_span_includes_leading_blank() {
        let lines = vec![
            "# Piano: 1".to_string(),
            "seq | C4 |".to_string(),
            "".to_string(),
            "# @riff".to_string(),
            "seq | C4 . |".to_string(),
            "kick | ^ . |".to_string(),
            "".to_string(),
            "# Bass: 2".to_string(),
        ];

        assert_eq!(
            template_definition_delete_span(&lines, 4),
            Some((2, 6, 3, "riff".to_string()))
        );
        assert_eq!(
            template_definition_delete_span(&lines, 5),
            Some((2, 6, 3, "riff".to_string()))
        );
        assert_eq!(template_definition_delete_span(&lines, 7), None);
    }

    #[test]
    fn template_call_macro_delete_removes_selected_macro() {
        let line = "[@riff +12 arp rev]*2";
        let call = template_call_spans_in_line(0, line).remove(0);

        assert_eq!(
            template_call_text_without_macro_at_cursor(call.start_col + 12, &call),
            Some(("[@riff +12 rev]*2".to_string(), "arp"))
        );
    }
}
