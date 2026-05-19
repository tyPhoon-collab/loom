use super::input::{PendingInput, ADD_HELP};
use super::selection::{
    bar_at_or_near_col, bar_spans_in_line, insert_at_col, is_seq_line, replace_char_range,
    unit_at_or_near_col, unit_spans_in_line, UnitSpan,
};
use super::settings::parse_track_header_channel;
use super::StudioApp;
use crossterm::event::{KeyCode, KeyEvent};
use miette::Result;

struct PlacedSlot {
    index_on_line: usize,
}

impl StudioApp {
    pub(super) fn handle_add_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.status_message = "Add cancelled".into();
            }
            KeyCode::Char('s') => {
                self.add_seq_line()?;
            }
            KeyCode::Char('l') => {
                self.add_note_head_line()?;
            }
            KeyCode::Char('t') => {
                self.add_track()?;
            }
            KeyCode::Char('d') => {
                self.add_default_drum_lanes()?;
            }
            KeyCode::Char('v') => {
                self.add_modifier_line("v")?;
            }
            KeyCode::Char('p') => {
                self.add_modifier_line("p")?;
            }
            KeyCode::Char('m') => {
                if self.current_template_call_at_cursor().is_some() {
                    self.begin_pending_input(PendingInput::TemplateMacro);
                } else {
                    self.status_message =
                        "Template macro add needs the cursor on a template call".into();
                }
            }
            KeyCode::Char('b') => {
                self.add_bar()?;
            }
            KeyCode::Char('n') => {
                let token = self.note_token_for_add();
                self.place_token_at_current_slot(&token)?;
            }
            KeyCode::Char('.') => {
                self.place_token_at_current_slot(".")?;
            }
            KeyCode::Char('-') => {
                self.place_token_at_current_slot("-")?;
            }
            _ => {
                self.status_message = format!("Unknown add command. {}", ADD_HELP);
            }
        }
        Ok(())
    }

    pub(super) fn add_default_drum_lanes(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let insert_row = insert_row_after_cursor(&lines, cursor.0);
        let inserted = vec![
            String::new(),
            "# Drums: 10".to_string(),
            "kick  | . . . . |".to_string(),
            "snare | . . . . |".to_string(),
            "hh    | . . . . |".to_string(),
            "oh    | . . . . |".to_string(),
        ];
        lines.splice(insert_row..insert_row, inserted);
        self.apply_cursor_source_update(
            lines,
            (insert_row + 2, 8),
            "Added default drum lanes".into(),
            None,
        )
    }

    pub(super) fn add_seq_line(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let insert_row = insert_row_after_cursor(&lines, cursor.0);
        lines.insert(insert_row, "seq | . . . . |".to_string());
        self.apply_cursor_source_update(lines, (insert_row, 6), "Added seq line".into(), None)
    }

    pub(super) fn add_note_head_line(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let insert_row = insert_row_after_cursor(&lines, cursor.0);
        let note = self.note_token_for_add();
        lines.insert(insert_row, format!("{} | ^ . . . |", note));
        self.apply_cursor_source_update(
            lines,
            (insert_row, 0),
            format!("Added note-head line: {}", note),
            Some((insert_row, note)),
        )
    }

    pub(super) fn add_track(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let insert_row = insert_row_after_cursor(&lines, cursor.0);
        let (track_name, channel) = next_track_header(&lines);
        let inserted = vec![
            String::new(),
            format!("# {}: {}", track_name, channel),
            "seq | . . . . |".to_string(),
        ];
        lines.splice(insert_row..insert_row, inserted);
        self.apply_cursor_source_update(
            lines,
            (insert_row + 2, 6),
            format!("Added track: {} on channel {}", track_name, channel),
            None,
        )
    }

    pub(super) fn add_bar(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(cursor.0) else {
            self.status_message = "No current line".into();
            return Ok(());
        };

        let Ok(new_cursor_col) = add_rest_bar_to_line(line) else {
            self.status_message = "Add bar needs a line with bars".into();
            return Ok(());
        };
        self.apply_cursor_source_update(lines, (cursor.0, new_cursor_col), "Added bar".into(), None)
    }

    pub(super) fn add_modifier_line(&mut self, label: &str) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(insert_row) = modifier_insert_row(&lines, cursor.0) else {
            self.status_message =
                "Modifier line add needs a seq, note-head, or drum lane block".into();
            return Ok(());
        };

        let template = format!("{:<2} | . . . . |", label);
        lines.insert(insert_row, template);
        self.apply_cursor_source_update(
            lines,
            (insert_row, 5),
            format!("Added {} modifier line", modifier_label_name(label)),
            None,
        )
    }

    pub(super) fn place_token_at_current_slot(&mut self, token: &str) -> Result<bool> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(cursor.0) else {
            self.status_message = "No current line".into();
            return Ok(false);
        };

        let Ok(slot) = place_seq_token_at_slot(cursor.0, line, cursor.1, token) else {
            self.status_message = "Place token currently supports seq lines only".into();
            return Ok(false);
        };

        let cursor_col = unit_spans_in_line(cursor.0, line)
            .get(slot.index_on_line)
            .map(|note| note.start_col)
            .unwrap_or(cursor.1);
        let audition = (token != "." && token != "-").then(|| (cursor.0, token.to_string()));
        self.apply_cursor_source_update(
            lines,
            (cursor.0, cursor_col),
            format!("Placed {}", token),
            audition,
        )?;
        Ok(true)
    }

    pub(super) fn note_token_for_add(&self) -> String {
        let cursor = self.textarea.cursor();
        if let Some(note) = unit_at_or_near_col(
            self.auditionable_spans_in_line(self.textarea.lines(), cursor.0),
            cursor.1,
        ) {
            return note.token;
        }

        self.textarea
            .lines()
            .iter()
            .enumerate()
            .take(cursor.0 + 1)
            .rev()
            .find_map(|(row, _)| {
                self.auditionable_spans_in_line(self.textarea.lines(), row)
                    .into_iter()
                    .next_back()
            })
            .map(|note| note.token)
            .unwrap_or_else(|| "C4".to_string())
    }
}

fn insert_row_after_cursor(lines: &[String], cursor_row: usize) -> usize {
    if lines.is_empty() {
        0
    } else {
        (cursor_row + 1).min(lines.len())
    }
}

fn modifier_insert_row(lines: &[String], cursor_row: usize) -> Option<usize> {
    let anchor_row = if lines
        .get(cursor_row)
        .is_some_and(|line| is_pattern_line(line) || is_modifier_line(line))
    {
        cursor_row
    } else {
        (0..=cursor_row).rev().find(|&row| {
            lines
                .get(row)
                .is_some_and(|line| is_pattern_line(line) || is_modifier_line(line))
        })?
    };

    if lines
        .get(anchor_row)
        .is_some_and(|line| is_modifier_line(line))
    {
        let pattern_row = (0..anchor_row)
            .rev()
            .find(|&row| lines.get(row).is_some_and(|line| is_pattern_line(line)))?;
        return Some(first_non_modifier_row_after(lines, pattern_row));
    }

    Some(first_non_modifier_row_after(lines, anchor_row))
}

fn first_non_modifier_row_after(lines: &[String], row: usize) -> usize {
    let mut insert_row = row + 1;
    while insert_row < lines.len() && is_modifier_line(&lines[insert_row]) {
        insert_row += 1;
    }
    insert_row
}

fn is_pattern_line(line: &str) -> bool {
    line.contains('|') && !is_modifier_line(line) && !line.trim().starts_with("[@")
}

fn is_modifier_line(line: &str) -> bool {
    let Some(pipe_col) = line.chars().position(|ch| ch == '|') else {
        return false;
    };
    matches!(
        line.chars().take(pipe_col).collect::<String>().trim(),
        "v" | "p"
    )
}

fn modifier_label_name(label: &str) -> &'static str {
    match label {
        "v" => "velocity",
        "p" => "pitch",
        _ => "unknown",
    }
}

fn next_track_header(lines: &[String]) -> (String, u8) {
    let track_count = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with('#') && !trimmed.starts_with("##")
        })
        .count();
    let next_channel = lines
        .iter()
        .filter_map(|line| parse_track_header_channel(line))
        .map(|channel| channel + 2)
        .max()
        .unwrap_or(1)
        .min(16);
    (format!("Track {}", track_count + 1), next_channel)
}

fn add_rest_bar_to_line(line: &mut String) -> std::result::Result<usize, ()> {
    if !line.contains('|') {
        return Err(());
    }
    let trimmed_len = line.trim_end().chars().count();
    replace_char_range(line, trimmed_len, line.chars().count(), "");
    let cursor_col = line.chars().count() + 1;
    line.push_str(" . . . . |");
    Ok(cursor_col)
}

fn place_seq_token_at_slot(
    row: usize,
    line: &mut String,
    col: usize,
    token: &str,
) -> std::result::Result<PlacedSlot, ()> {
    if !is_seq_line(line) {
        return Err(());
    }

    let notes = unit_spans_in_line(row, line);
    if let Some((index, note)) = unit_at_or_near_col_with_index(&notes, col) {
        replace_char_range(line, note.start_col, note.end_col, token);
        return Ok(PlacedSlot {
            index_on_line: index,
        });
    }

    let Some(bar) = bar_at_or_near_col(bar_spans_in_line(row, line), col) else {
        return Err(());
    };
    let insertion_col = bar.start_col + 1;
    let chars: Vec<char> = line.chars().collect();
    let insertion = if chars
        .get(insertion_col)
        .is_some_and(|ch| ch.is_whitespace())
    {
        format!(" {}", token)
    } else {
        format!(" {} ", token)
    };
    insert_at_col(line, insertion_col, &insertion);
    Ok(PlacedSlot { index_on_line: 0 })
}

fn unit_at_or_near_col_with_index(notes: &[UnitSpan], col: usize) -> Option<(usize, UnitSpan)> {
    notes
        .iter()
        .enumerate()
        .find(|(_, note)| col >= note.start_col && col < note.end_col)
        .map(|(index, note)| (index, note.clone()))
        .or_else(|| {
            notes
                .iter()
                .enumerate()
                .find(|(_, note)| note.start_col >= col)
                .map(|(index, note)| (index, note.clone()))
        })
        .or_else(|| {
            notes
                .iter()
                .enumerate()
                .next_back()
                .map(|(index, note)| (index, note.clone()))
        })
}

#[cfg(test)]
mod tests {
    use super::{add_rest_bar_to_line, next_track_header, place_seq_token_at_slot};

    #[test]
    pub(super) fn add_rest_bar_appends_grid_bar() {
        let mut line = "seq | C4 . |".to_string();
        let cursor_col = add_rest_bar_to_line(&mut line).unwrap();
        assert_eq!(line, "seq | C4 . | . . . . |");
        assert_eq!(cursor_col, 13);
    }

    #[test]
    pub(super) fn place_seq_token_replaces_current_slot() {
        let mut line = "seq | C4 . E4 |".to_string();
        let placed = place_seq_token_at_slot(0, &mut line, 9, "D4").unwrap();
        assert_eq!(line, "seq | C4 D4 E4 |");
        assert_eq!(placed.index_on_line, 1);
    }

    #[test]
    pub(super) fn place_seq_token_in_empty_bar() {
        let mut line = "seq | |".to_string();
        let placed = place_seq_token_at_slot(0, &mut line, 5, "C4").unwrap();
        assert_eq!(line, "seq | C4 |");
        assert_eq!(placed.index_on_line, 0);
    }

    #[test]
    pub(super) fn next_track_header_uses_next_track_number_and_channel() {
        let lines = vec![
            "# Piano: 1".to_string(),
            "seq | C4 |".to_string(),
            "# Bass: 3".to_string(),
        ];
        assert_eq!(next_track_header(&lines), ("Track 3".to_string(), 4));
    }
}
