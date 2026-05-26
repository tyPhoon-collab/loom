use super::input::{NoteInputMode, PendingInput, ONSET_HELP};
use super::keystroke::{
    key_stroke_matches, lookup_key_action, normalized_key_stroke, KeyBinding, KeyStroke,
};
use super::selection::{
    bar_at_or_near_col, bar_spans_in_line, insert_at_col, is_lane_body_token,
    lane_body_token_spans_in_line, lane_head_token, replace_char_range, UnitSpan,
};
use super::StudioApp;
use crossterm::event::{KeyCode, KeyEvent};
use miette::Result;

struct PlacedOnset {
    index_on_line: usize,
    token: char,
}

#[derive(Clone, Copy, Debug)]
enum OnsetKeyAction {
    UndoContinuous,
    Cancel,
    PlaceNoteOn,
    PlaceRest,
    PlaceSustain,
    Toggle,
}

const ONSET_KEY_BINDINGS: &[KeyBinding<OnsetKeyAction>] = &[
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Backspace),
        action: OnsetKeyAction::UndoContinuous,
    },
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Esc),
        action: OnsetKeyAction::Cancel,
    },
    KeyBinding {
        stroke: KeyStroke::Char('x'),
        action: OnsetKeyAction::PlaceNoteOn,
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('.'),
        action: OnsetKeyAction::PlaceRest,
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('-'),
        action: OnsetKeyAction::PlaceSustain,
    },
    KeyBinding {
        stroke: KeyStroke::Char('t'),
        action: OnsetKeyAction::Toggle,
    },
];

impl StudioApp {
    pub(super) fn handle_onset_key(&mut self, mode: NoteInputMode, key: KeyEvent) -> Result<()> {
        let pending = PendingInput::Onset(mode);

        match normalized_key_stroke(&key) {
            Some(KeyStroke::Char(' ')) if matches!(mode, NoteInputMode::Continuous) => {
                self.skip_current_continuous_input(pending);
                return Ok(());
            }
            Some(KeyStroke::Code(KeyCode::Tab)) if matches!(mode, NoteInputMode::Continuous) => {
                self.subdivide_current_unit()?;
                self.resume_continuous_input(pending);
                return Ok(());
            }
            Some(KeyStroke::Code(KeyCode::BackTab))
                if matches!(mode, NoteInputMode::Continuous) =>
            {
                self.shrink_current_editable_group()?;
                self.resume_continuous_input(pending);
                return Ok(());
            }
            Some(KeyStroke::Code(KeyCode::Backspace))
                if matches!(mode, NoteInputMode::Continuous) =>
            {
                self.handle_continuous_input_undo(pending)?;
                return Ok(());
            }
            _ => {}
        }

        let Some(action) = lookup_key_action(ONSET_KEY_BINDINGS, &key) else {
            if pending.is_continuous() {
                self.reject_pending_input(pending);
            } else {
                self.status_message = pending.unknown_message();
            }
            return Ok(());
        };

        match action {
            OnsetKeyAction::UndoContinuous => {
                if pending.is_continuous() {
                    self.handle_continuous_input_undo(pending)?;
                } else {
                    self.status_message = pending.unknown_message();
                }
            }
            OnsetKeyAction::Cancel => self.cancel_pending_input(pending),
            OnsetKeyAction::PlaceNoteOn => self.apply_onset_entry(pending, '^')?,
            OnsetKeyAction::PlaceRest => self.apply_onset_entry(pending, '.')?,
            OnsetKeyAction::PlaceSustain => self.apply_onset_entry(pending, '-')?,
            OnsetKeyAction::Toggle => self.apply_toggled_onset_entry(pending)?,
        }
        Ok(())
    }

    pub(super) fn handle_select_onset_key(&mut self, key: KeyEvent) -> Result<()> {
        if key_stroke_matches(KeyStroke::Code(KeyCode::Esc), &key) {
            self.status_message = "Onset edit cancelled".into();
            return Ok(());
        }

        let Some(action) = lookup_key_action(ONSET_KEY_BINDINGS, &key) else {
            self.status_message = format!("Unknown onset command. {}", ONSET_HELP);
            return Ok(());
        };

        match action {
            OnsetKeyAction::Cancel => {
                self.status_message = "Onset edit cancelled".into();
            }
            OnsetKeyAction::PlaceNoteOn => {
                self.replace_selected_onset_tokens('^')?;
            }
            OnsetKeyAction::PlaceRest => {
                self.replace_selected_onset_tokens('.')?;
            }
            OnsetKeyAction::PlaceSustain => {
                self.replace_selected_onset_tokens('-')?;
            }
            OnsetKeyAction::Toggle => {
                self.toggle_selected_onset_tokens()?;
            }
            OnsetKeyAction::UndoContinuous => {
                self.status_message = format!("Unknown onset command. {}", ONSET_HELP);
            }
        }
        Ok(())
    }

    pub(super) fn place_onset_token_at_current_slot(&mut self, token: char) -> Result<bool> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(cursor.0) else {
            self.status_message = "No current line".into();
            return Ok(false);
        };

        let Ok(placed) = place_lane_onset_at_slot(cursor.0, line, cursor.1, token) else {
            self.status_message = "Onset edit needs a note-head or drum lane line".into();
            return Ok(false);
        };
        let audition = if placed.token == '^' {
            lane_head_token(line).map(|head| (cursor.0, head))
        } else {
            None
        };
        let cursor_col = lane_body_token_spans_in_line(cursor.0, line)
            .get(placed.index_on_line)
            .map(|onset| onset.start_col)
            .unwrap_or(cursor.1);
        self.apply_cursor_source_update(
            lines,
            (cursor.0, cursor_col),
            format!("Placed onset {}", placed.token),
            audition,
        )?;
        Ok(true)
    }

    pub(super) fn toggle_onset_token_at_current_slot(&mut self) -> Result<bool> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(cursor.0) else {
            self.status_message = "No current line".into();
            return Ok(false);
        };

        let token = current_onset_token(line, cursor.1)
            .map(|span| if span.token == "^" { '.' } else { '^' })
            .unwrap_or('^');
        let Ok(placed) = place_lane_onset_at_slot(cursor.0, line, cursor.1, token) else {
            self.status_message = "Onset toggle needs a note-head or drum lane line".into();
            return Ok(false);
        };
        let audition = if placed.token == '^' {
            lane_head_token(line).map(|head| (cursor.0, head))
        } else {
            None
        };
        let cursor_col = lane_body_token_spans_in_line(cursor.0, line)
            .get(placed.index_on_line)
            .map(|onset| onset.start_col)
            .unwrap_or(cursor.1);
        self.apply_cursor_source_update(
            lines,
            (cursor.0, cursor_col),
            format!("Toggled onset to {}", placed.token),
            audition,
        )?;
        Ok(true)
    }

    fn apply_onset_entry(&mut self, pending: PendingInput, token: char) -> Result<()> {
        let placed = self.place_onset_token_at_current_slot(token)?;
        self.resume_continuous_onset_if_needed(pending, placed);
        Ok(())
    }

    fn apply_toggled_onset_entry(&mut self, pending: PendingInput) -> Result<()> {
        let placed = self.toggle_onset_token_at_current_slot()?;
        self.resume_continuous_onset_if_needed(pending, placed);
        Ok(())
    }

    fn resume_continuous_onset_if_needed(&mut self, pending: PendingInput, placed: bool) {
        if !pending.is_continuous() {
            return;
        }
        if placed {
            self.advance_after_continuous_edit(pending);
        } else {
            self.resume_continuous_input(pending);
        }
    }

    pub(super) fn replace_selected_onset_tokens(&mut self, token: char) -> Result<()> {
        let selected_indices = self.selected_unit_indices();
        let mut selected_tokens = self.selected_unit_spans();
        if selected_tokens.is_empty() {
            self.status_message = "Onset edit applies to unit selection only".into();
            return Ok(());
        }

        let lines = self.textarea.lines();
        if selected_tokens.iter().any(|selected| {
            lines
                .get(selected.row)
                .is_none_or(|line| !is_lane_body_token(line, selected))
        }) {
            self.status_message = "Onset edit applies to lane body unit selection only".into();
            return Ok(());
        }

        let audition = if token == '^' {
            selected_tokens.first().and_then(|selected| {
                lines
                    .get(selected.row)
                    .and_then(|line| lane_head_token(line).map(|head| (selected.row, head)))
            })
        } else {
            None
        };

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
                &token.to_string(),
            );
        }

        self.apply_unit_selection_update(
            lines,
            &selected_indices,
            format!(
                "Set onset {} on {} unit{}",
                token,
                selected_tokens.len(),
                if selected_tokens.len() == 1 { "" } else { "s" }
            ),
            audition,
        )
    }

    pub(super) fn toggle_selected_onset_tokens(&mut self) -> Result<()> {
        let selected_indices = self.selected_unit_indices();
        let mut selected_tokens = self.selected_unit_spans();
        if selected_tokens.is_empty() {
            self.status_message = "Onset toggle applies to unit selection only".into();
            return Ok(());
        }

        let lines = self.textarea.lines();
        if selected_tokens.iter().any(|selected| {
            lines
                .get(selected.row)
                .is_none_or(|line| !is_lane_body_token(line, selected))
        }) {
            self.status_message = "Onset toggle applies to lane body unit selection only".into();
            return Ok(());
        }

        selected_tokens.sort_by(|left, right| {
            right
                .row
                .cmp(&left.row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });

        let mut lines = lines.to_vec();
        let mut audition = None;
        for selected in &selected_tokens {
            let Some(line) = lines.get_mut(selected.row) else {
                self.status_message = "Selected unit no longer exists".into();
                return Ok(());
            };
            let token = toggled_onset_token(&selected.token);
            if audition.is_none() && token == '^' {
                audition = lane_head_token(line).map(|head| (selected.row, head));
            }
            replace_char_range(
                line,
                selected.start_col,
                selected.end_col,
                &token.to_string(),
            );
        }

        self.apply_unit_selection_update(
            lines,
            &selected_indices,
            format!(
                "Toggled onset on {} unit{}",
                selected_tokens.len(),
                if selected_tokens.len() == 1 { "" } else { "s" }
            ),
            audition,
        )
    }
}

fn place_lane_onset_at_slot(
    row: usize,
    line: &mut String,
    col: usize,
    token: char,
) -> std::result::Result<PlacedOnset, ()> {
    if lane_head_token(line).is_none() {
        return Err(());
    }

    let onsets = lane_body_token_spans_in_line(row, line);
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

fn current_onset_token(line: &str, col: usize) -> Option<UnitSpan> {
    let onsets = lane_body_token_spans_in_line(0, line);
    onset_at_or_near_col_with_index(&onsets, col).map(|(_, onset)| onset)
}

fn toggled_onset_token(token: &str) -> char {
    if token == "^" {
        '.'
    } else {
        '^'
    }
}

fn onset_at_or_near_col_with_index(onsets: &[UnitSpan], col: usize) -> Option<(usize, UnitSpan)> {
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

#[cfg(test)]
mod tests {
    use super::{place_lane_onset_at_slot, toggled_onset_token};

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

    #[test]
    fn toggle_onset_uses_note_on_for_rest_or_sustain() {
        assert_eq!(toggled_onset_token("^"), '.');
        assert_eq!(toggled_onset_token("."), '^');
        assert_eq!(toggled_onset_token("-"), '^');
    }
}
