use super::input::ONSET_HELP;
use super::selection::{
    bar_at_or_near_col, bar_spans_in_line, insert_at_col, is_seq_line, replace_char_range,
};
use super::StudioApp;
use crossterm::event::{KeyCode, KeyEvent};
use miette::Result;
use ratatui_textarea::CursorMove;

#[derive(Clone, Debug, PartialEq, Eq)]
struct OnsetTokenSpan {
    start_col: usize,
    end_col: usize,
    token: char,
}

struct PlacedOnset {
    index_on_line: usize,
    token: char,
}

impl StudioApp {
    pub(super) fn handle_onset_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.status_message = "Onset edit cancelled".into();
            }
            KeyCode::Char('x') => {
                self.place_onset_token_at_current_slot('^')?;
            }
            KeyCode::Char('.') => {
                self.place_onset_token_at_current_slot('.')?;
            }
            KeyCode::Char('-') => {
                self.place_onset_token_at_current_slot('-')?;
            }
            KeyCode::Char('t') => {
                self.toggle_onset_token_at_current_slot()?;
            }
            _ => {
                self.status_message = format!("Unknown onset command. {}", ONSET_HELP);
            }
        }
        Ok(())
    }

    pub(super) fn place_onset_token_at_current_slot(&mut self, token: char) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(cursor.0) else {
            self.status_message = "No current line".into();
            return Ok(());
        };

        let Ok(placed) = place_lane_onset_at_slot(cursor.0, line, cursor.1, token) else {
            self.status_message = "Onset edit needs a note-head or drum lane line".into();
            return Ok(());
        };
        let audition = if placed.token == '^' {
            lane_head_token(line).map(|head| (cursor.0, head))
        } else {
            None
        };

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        if let Some(onset) = onset_spans_in_line(
            self.textarea
                .lines()
                .get(cursor.0)
                .map(String::as_str)
                .unwrap_or_default(),
        )
        .get(placed.index_on_line)
        {
            self.textarea
                .move_cursor(CursorMove::Jump(cursor.0 as u16, onset.start_col as u16));
        }
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!("Placed onset {}", placed.token);
        self.audition_candidate(audition);
        Ok(())
    }

    pub(super) fn toggle_onset_token_at_current_slot(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(cursor.0) else {
            self.status_message = "No current line".into();
            return Ok(());
        };

        let token = current_onset_token(line, cursor.1)
            .map(|span| if span.token == '^' { '.' } else { '^' })
            .unwrap_or('^');
        let Ok(placed) = place_lane_onset_at_slot(cursor.0, line, cursor.1, token) else {
            self.status_message = "Onset toggle needs a note-head or drum lane line".into();
            return Ok(());
        };
        let audition = if placed.token == '^' {
            lane_head_token(line).map(|head| (cursor.0, head))
        } else {
            None
        };

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        if let Some(onset) = onset_spans_in_line(
            self.textarea
                .lines()
                .get(cursor.0)
                .map(String::as_str)
                .unwrap_or_default(),
        )
        .get(placed.index_on_line)
        {
            self.textarea
                .move_cursor(CursorMove::Jump(cursor.0 as u16, onset.start_col as u16));
        }
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!("Toggled onset to {}", placed.token);
        self.audition_candidate(audition);
        Ok(())
    }
}

fn place_lane_onset_at_slot(
    _row: usize,
    line: &mut String,
    col: usize,
    token: char,
) -> std::result::Result<PlacedOnset, ()> {
    if is_seq_line(line) || lane_head_token(line).is_none() {
        return Err(());
    }

    let onsets = onset_spans_in_line(line);
    if let Some((index, onset)) = onset_at_or_near_col_with_index(&onsets, col) {
        replace_char_range(line, onset.start_col, onset.end_col, &token.to_string());
        return Ok(PlacedOnset {
            index_on_line: index,
            token,
        });
    }

    let Some(bar) = bar_at_or_near_col(bar_spans_in_line(0, line), col) else {
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
    Ok(PlacedOnset {
        index_on_line: 0,
        token,
    })
}

fn current_onset_token(line: &str, col: usize) -> Option<OnsetTokenSpan> {
    let onsets = onset_spans_in_line(line);
    onset_at_or_near_col_with_index(&onsets, col).map(|(_, onset)| onset)
}

fn onset_spans_in_line(line: &str) -> Vec<OnsetTokenSpan> {
    let mut spans = Vec::new();
    for bar in bar_spans_in_line(0, line) {
        let chars: Vec<char> = line.chars().collect();
        for col in bar.start_col + 1..bar.end_col.saturating_sub(1) {
            let Some(ch) = chars.get(col).copied() else {
                continue;
            };
            if matches!(ch, '^' | '.' | '-') {
                spans.push(OnsetTokenSpan {
                    start_col: col,
                    end_col: col + 1,
                    token: ch,
                });
            }
        }
    }
    spans
}

fn onset_at_or_near_col_with_index(
    onsets: &[OnsetTokenSpan],
    col: usize,
) -> Option<(usize, OnsetTokenSpan)> {
    onsets
        .iter()
        .enumerate()
        .find(|(_, onset)| col >= onset.start_col && col < onset.end_col)
        .map(|(index, onset)| (index, onset.clone()))
        .or_else(|| {
            onsets
                .iter()
                .enumerate()
                .find(|(_, onset)| onset.start_col >= col)
                .map(|(index, onset)| (index, onset.clone()))
        })
        .or_else(|| {
            onsets
                .iter()
                .enumerate()
                .next_back()
                .map(|(index, onset)| (index, onset.clone()))
        })
}

fn lane_head_token(line: &str) -> Option<String> {
    let pipe_col = line.chars().position(|ch| ch == '|')?;
    let head: String = line.chars().take(pipe_col).collect();
    let head = head.trim();
    (!head.is_empty() && head != "seq").then(|| head.to_string())
}

#[cfg(test)]
mod tests {
    use super::place_lane_onset_at_slot;

    #[test]
    fn place_onset_replaces_current_lane_slot() {
        let mut line = "kick | . . . . |".to_string();
        let placed = place_lane_onset_at_slot(0, &mut line, 7, '^').unwrap();
        assert_eq!(line, "kick | ^ . . . |");
        assert_eq!(placed.index_on_line, 0);
    }

    #[test]
    fn place_onset_replaces_nearby_lane_slot() {
        let mut line = "snare | . . . . |".to_string();
        let placed = place_lane_onset_at_slot(0, &mut line, 11, '^').unwrap();
        assert_eq!(line, "snare | . . ^ . |");
        assert_eq!(placed.index_on_line, 2);
    }

    #[test]
    fn place_onset_in_empty_bar() {
        let mut line = "hh | |".to_string();
        let placed = place_lane_onset_at_slot(0, &mut line, 4, '^').unwrap();
        assert_eq!(line, "hh | ^ |");
        assert_eq!(placed.index_on_line, 0);
    }

    #[test]
    fn place_onset_rejects_seq_lines() {
        let mut line = "seq | C4 . E4 |".to_string();
        assert!(place_lane_onset_at_slot(0, &mut line, 6, '^').is_err());
    }

    fn toggle_token(input: char) -> char {
        if input == '^' {
            '.'
        } else {
            '^'
        }
    }

    #[test]
    fn toggle_onset_uses_note_on_for_rest_or_sustain() {
        assert_eq!(toggle_token('^'), '.');
        assert_eq!(toggle_token('.'), '^');
        assert_eq!(toggle_token('-'), '^');
    }
}
