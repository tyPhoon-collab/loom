use super::input::PendingInput;
use super::keystroke::{
    key_stroke_matches, lookup_key_action, normalized_key_stroke, KeyBinding, KeyStroke,
};
use super::selection::{
    bar_at_or_near_col, bar_spans_in_line, is_seq_line, lane_head_token, replace_char_range,
};
use super::settings::{format_track_header, parse_track_header};
use super::StudioApp;
use crate::dsl::token::TrackInitLabel;
use crossterm::event::{KeyCode, KeyEvent};
use miette::Result;
use ratatui_textarea::CursorMove;

#[derive(Clone, Copy, Debug)]
enum GotoKeyAction {
    Cancel,
    NextTrack,
    PreviousTrack,
    GotoDefinition,
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
    BeginTrackInitDelete,
}

const GOTO_KEY_BINDINGS: &[KeyBinding<GotoKeyAction>] = &[
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Esc),
        action: GotoKeyAction::Cancel,
    },
    KeyBinding {
        stroke: KeyStroke::Char('t'),
        action: GotoKeyAction::NextTrack,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('t'),
        action: GotoKeyAction::PreviousTrack,
    },
    KeyBinding {
        stroke: KeyStroke::Char('d'),
        action: GotoKeyAction::GotoDefinition,
    },
];

const DELETE_STRUCTURE_KEY_BINDINGS: &[KeyBinding<DeleteStructureKeyAction>] = &[
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Esc),
        action: DeleteStructureKeyAction::Cancel,
    },
    KeyBinding {
        stroke: KeyStroke::Char('s'),
        action: DeleteStructureKeyAction::DeleteSeqLine,
    },
    KeyBinding {
        stroke: KeyStroke::Char('l'),
        action: DeleteStructureKeyAction::DeleteNoteHeadLine,
    },
    KeyBinding {
        stroke: KeyStroke::Char('t'),
        action: DeleteStructureKeyAction::DeleteTrack,
    },
    KeyBinding {
        stroke: KeyStroke::Char('h'),
        action: DeleteStructureKeyAction::DeleteSeparator,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('t'),
        action: DeleteStructureKeyAction::DeleteTemplateDefinition,
    },
    KeyBinding {
        stroke: KeyStroke::Char('b'),
        action: DeleteStructureKeyAction::DeleteBar,
    },
    KeyBinding {
        stroke: KeyStroke::Char('v'),
        action: DeleteStructureKeyAction::DeleteVelocityModifier,
    },
    KeyBinding {
        stroke: KeyStroke::Char('p'),
        action: DeleteStructureKeyAction::DeletePitchModifier,
    },
    KeyBinding {
        stroke: KeyStroke::Char('m'),
        action: DeleteStructureKeyAction::BeginDeleteTemplateMacro,
    },
    KeyBinding {
        stroke: KeyStroke::Char('i'),
        action: DeleteStructureKeyAction::BeginTrackInitDelete,
    },
];

impl StudioApp {
    pub(super) fn handle_goto_key(&mut self, key: KeyEvent) -> Result<()> {
        let Some(action) = lookup_key_action(GOTO_KEY_BINDINGS, &key) else {
            self.reject_pending_input(PendingInput::Goto);
            return Ok(());
        };

        match action {
            GotoKeyAction::Cancel => self.cancel_pending_input(PendingInput::Goto),
            GotoKeyAction::NextTrack => self.goto_adjacent_track(1),
            GotoKeyAction::PreviousTrack => self.goto_adjacent_track(-1),
            GotoKeyAction::GotoDefinition => self.goto_current_definition()?,
        }
        Ok(())
    }

    pub(super) fn handle_delete_structure_key(&mut self, key: KeyEvent) -> Result<()> {
        let Some(action) = lookup_key_action(DELETE_STRUCTURE_KEY_BINDINGS, &key) else {
            self.reject_pending_input(PendingInput::DeleteStructure);
            return Ok(());
        };

        match action {
            DeleteStructureKeyAction::Cancel => {
                self.cancel_pending_input(PendingInput::DeleteStructure)
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
            DeleteStructureKeyAction::BeginTrackInitDelete => {
                self.begin_pending_input(PendingInput::TrackInitDelete)
            }
        }
        Ok(())
    }

    pub(super) fn handle_track_init_delete_key(&mut self, key: KeyEvent) -> Result<()> {
        let Some(spec) = parse_track_init_key(key) else {
            self.reject_pending_input(PendingInput::TrackInitDelete);
            return Ok(());
        };

        if matches!(spec, TrackInitKeySpec::Cancel) {
            self.cancel_pending_input(PendingInput::TrackInitDelete);
            return Ok(());
        }

        self.delete_track_init_line(spec)
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

    fn delete_track_init_line(&mut self, spec: TrackInitKeySpec) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(header_row) = current_track_header_row(&lines, cursor.0) else {
            self.status_message = "Track init delete needs the cursor inside a track".into();
            return Ok(());
        };
        let Some(row) = find_track_init_row(&lines, header_row, cursor.0, spec) else {
            self.status_message =
                format!("No {} init line found in the current track", spec.label());
            return Ok(());
        };

        lines.remove(row);
        let next_row = row.min(lines.len().saturating_sub(1));
        self.apply_cursor_source_update(
            lines,
            (next_row, 0),
            format!("Deleted track init {}", spec.label()),
            None,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrackInitKeySpec {
    Cancel,
    Pc,
    Bank,
    Cc,
    Pan,
    Volume,
    Expression,
    Mod,
    Sustain,
}

impl TrackInitKeySpec {
    fn label(self) -> &'static str {
        match self {
            TrackInitKeySpec::Cancel => "cancel",
            TrackInitKeySpec::Pc => "pc",
            TrackInitKeySpec::Bank => "bank",
            TrackInitKeySpec::Cc => "cc",
            TrackInitKeySpec::Pan => "pan",
            TrackInitKeySpec::Volume => "volume",
            TrackInitKeySpec::Expression => "expression",
            TrackInitKeySpec::Mod => "mod",
            TrackInitKeySpec::Sustain => "sustain",
        }
    }
}

fn parse_track_init_key(key: KeyEvent) -> Option<TrackInitKeySpec> {
    if key_stroke_matches(KeyStroke::Code(KeyCode::Esc), &key) {
        return Some(TrackInitKeySpec::Cancel);
    }

    match normalized_key_stroke(&key) {
        Some(KeyStroke::Char(ch)) | Some(KeyStroke::ShiftChar(ch)) => match ch {
            'p' => Some(TrackInitKeySpec::Pc),
            'b' => Some(TrackInitKeySpec::Bank),
            'c' => Some(TrackInitKeySpec::Cc),
            'n' => Some(TrackInitKeySpec::Pan),
            'v' => Some(TrackInitKeySpec::Volume),
            'e' => Some(TrackInitKeySpec::Expression),
            'm' => Some(TrackInitKeySpec::Mod),
            's' => Some(TrackInitKeySpec::Sustain),
            _ => None,
        },
        _ => None,
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

fn current_track_header_row(lines: &[String], cursor_row: usize) -> Option<usize> {
    (0..=cursor_row)
        .rev()
        .find(|&row| {
            lines
                .get(row)
                .and_then(|line| parse_track_header(line))
                .is_some()
        })
        .or_else(|| {
            lines
                .iter()
                .position(|line| parse_track_header(line).is_some())
        })
}

fn is_track_init_line(line: &str) -> bool {
    line.trim_start().starts_with("## ")
}

fn track_init_label_of_line(line: &str) -> Option<TrackInitLabel> {
    let command = line.trim().strip_prefix("## ")?;
    let head = command.split_whitespace().next()?.to_ascii_lowercase();
    match head.as_str() {
        "pc" => Some(TrackInitLabel::Pc),
        "sound" => Some(TrackInitLabel::Sound),
        "bank" => Some(TrackInitLabel::Bank),
        "cc" => Some(TrackInitLabel::Cc),
        "pan" => Some(TrackInitLabel::Pan),
        "volume" => Some(TrackInitLabel::Volume),
        "expression" => Some(TrackInitLabel::Expression),
        "mod" => Some(TrackInitLabel::Mod),
        "sustain" => Some(TrackInitLabel::Sustain),
        _ => None,
    }
}

fn find_track_init_row(
    lines: &[String],
    header_row: usize,
    cursor_row: usize,
    spec: TrackInitKeySpec,
) -> Option<usize> {
    let label = match spec {
        TrackInitKeySpec::Cancel => return None,
        TrackInitKeySpec::Pc => TrackInitLabel::Pc,
        TrackInitKeySpec::Bank => TrackInitLabel::Bank,
        TrackInitKeySpec::Cc => TrackInitLabel::Cc,
        TrackInitKeySpec::Pan => TrackInitLabel::Pan,
        TrackInitKeySpec::Volume => TrackInitLabel::Volume,
        TrackInitKeySpec::Expression => TrackInitLabel::Expression,
        TrackInitKeySpec::Mod => TrackInitLabel::Mod,
        TrackInitKeySpec::Sustain => TrackInitLabel::Sustain,
    };

    let mut init_rows = Vec::new();
    let mut row = header_row + 1;
    while row < lines.len() && is_track_init_line(&lines[row]) {
        let row_label = track_init_label_of_line(&lines[row]);
        let matches_label = if spec == TrackInitKeySpec::Pc {
            matches!(row_label, Some(TrackInitLabel::Pc | TrackInitLabel::Sound))
        } else {
            row_label == Some(label)
        };
        if matches_label {
            init_rows.push(row);
        }
        row += 1;
    }

    if label == TrackInitLabel::Cc {
        init_rows.into_iter().find(|&row| row == cursor_row)
    } else {
        init_rows.into_iter().next()
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
        adjacent_track_header_row, current_track_index, find_track_init_row, modifier_line_kind,
        template_call_text_without_macro_at_cursor, template_definition_delete_span,
        track_delete_span, track_header_rows, track_init_label_of_line, TrackInitKeySpec,
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
    fn track_delete_span_can_cover_last_remaining_track() {
        let lines = vec!["# Piano: 1".to_string(), "seq | C4 |".to_string()];
        let headers = track_header_rows(&lines);
        assert_eq!(track_delete_span(&lines, &headers, 0), (0, 2, 0));
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

    #[test]
    fn track_init_label_of_line_parses_supported_labels() {
        assert_eq!(
            track_init_label_of_line("## pc 30"),
            Some(crate::dsl::token::TrackInitLabel::Pc)
        );
        assert_eq!(
            track_init_label_of_line("## pan 64"),
            Some(crate::dsl::token::TrackInitLabel::Pan)
        );
        assert_eq!(track_init_label_of_line("seq | C4 |"), None);
    }

    #[test]
    fn find_track_init_row_prefers_cursor_for_cc() {
        let lines = vec![
            "# Piano: 1".to_string(),
            "## cc 1 20".to_string(),
            "## cc 7 100".to_string(),
            "seq | C4 |".to_string(),
        ];

        assert_eq!(
            find_track_init_row(&lines, 0, 2, TrackInitKeySpec::Cc),
            Some(2)
        );
        assert_eq!(
            find_track_init_row(&lines, 0, 3, TrackInitKeySpec::Cc),
            None
        );
    }
}
