use crate::dsl::note::Note;

#[derive(Clone, Debug)]
pub(super) enum StudioSelection {
    Note {
        row: usize,
        start_col: usize,
        end_col: usize,
        token: String,
    },
    NoteRange {
        anchor: NoteTokenSpan,
        focus: NoteTokenSpan,
    },
    LineRange {
        anchor_row: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NoteTokenSpan {
    pub(super) row: usize,
    pub(super) start_col: usize,
    pub(super) end_col: usize,
    pub(super) token: String,
}

pub(super) fn note_spans_in_line(row: usize, line: &str) -> Vec<NoteTokenSpan> {
    let Some(pipe_col) = line.chars().position(|ch| ch == '|') else {
        return Vec::new();
    };

    let head: String = line.chars().take(pipe_col).collect();
    let scan_start_col = if head.trim() == "seq" {
        pipe_col + 1
    } else {
        0
    };
    let scan_end_col = if head.trim() == "seq" {
        line.chars().count()
    } else {
        pipe_col
    };

    let mut spans = Vec::new();
    let mut token = String::new();
    let mut token_start = 0usize;

    for (col, ch) in line.chars().enumerate() {
        if col < scan_start_col || col >= scan_end_col {
            if !token.is_empty() {
                push_note_span(&mut spans, row, token_start, col, &token);
                token.clear();
            }
            continue;
        }

        if is_note_token_char(ch) {
            if token.is_empty() {
                token_start = col;
            }
            token.push(ch);
        } else if !token.is_empty() {
            push_note_span(&mut spans, row, token_start, col, &token);
            token.clear();
        }
    }

    if !token.is_empty() {
        push_note_span(&mut spans, row, token_start, line.chars().count(), &token);
    }

    spans
}

pub(super) fn note_at_or_near_col(notes: Vec<NoteTokenSpan>, col: usize) -> Option<NoteTokenSpan> {
    notes
        .iter()
        .find(|note| col >= note.start_col && col < note.end_col)
        .cloned()
        .or_else(|| notes.iter().find(|note| note.start_col >= col).cloned())
        .or_else(|| notes.into_iter().next_back())
}

pub(super) fn ordered_note_span_bounds<'a>(
    left: &'a NoteTokenSpan,
    right: &'a NoteTokenSpan,
) -> (&'a NoteTokenSpan, &'a NoteTokenSpan) {
    if (left.row, left.start_col) <= (right.row, right.start_col) {
        (left, right)
    } else {
        (right, left)
    }
}

pub(super) fn replace_char_range(
    line: &mut String,
    start_col: usize,
    end_col: usize,
    replacement: &str,
) {
    let start = char_to_byte_index(line, start_col);
    let end = char_to_byte_index(line, end_col);
    line.replace_range(start..end, replacement);
}

fn push_note_span(
    spans: &mut Vec<NoteTokenSpan>,
    row: usize,
    start_col: usize,
    end_col: usize,
    token: &str,
) {
    let Ok(note) = token.parse::<Note>() else {
        return;
    };
    if matches!(note, Note::Drum(_)) {
        return;
    }

    spans.push(NoteTokenSpan {
        row,
        start_col,
        end_col,
        token: token.to_string(),
    });
}

pub(super) fn is_note_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '#' || ch == '-'
}

fn char_to_byte_index(input: &str, char_index: usize) -> usize {
    input
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(input.len())
}

#[cfg(test)]
mod tests {
    use super::{
        note_at_or_near_col, note_spans_in_line, ordered_note_span_bounds, replace_char_range,
    };

    #[test]
    fn note_spans_seq_body_only() {
        let spans = note_spans_in_line(0, "seq | D4 . Eb4 A#3 |");
        let tokens: Vec<_> = spans.iter().map(|span| span.token.as_str()).collect();
        assert_eq!(tokens, vec!["D4", "Eb4", "A#3"]);
    }

    #[test]
    fn note_spans_note_head_only() {
        let spans = note_spans_in_line(0, "F4,C5 | ^ . |");
        let tokens: Vec<_> = spans.iter().map(|span| span.token.as_str()).collect();
        assert_eq!(tokens, vec!["F4", "C5"]);
    }

    #[test]
    fn replace_selected_note_token() {
        let mut line = "seq | D4 . Eb4 |".to_string();
        let span = note_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "Eb4")
            .unwrap();
        replace_char_range(&mut line, span.start_col, span.end_col, "E4");
        assert_eq!(line, "seq | D4 . E4 |");
    }

    #[test]
    fn note_select_uses_note_under_cursor() {
        let notes = note_spans_in_line(0, "seq | D4 . Eb4 |");
        let selected = note_at_or_near_col(notes, 6).unwrap();
        assert_eq!(selected.token, "D4");
    }

    #[test]
    fn note_select_falls_forward() {
        let notes = note_spans_in_line(0, "seq | D4 . Eb4 |");
        let selected = note_at_or_near_col(notes, 9).unwrap();
        assert_eq!(selected.token, "Eb4");
    }

    #[test]
    fn note_select_falls_back_to_last_note() {
        let notes = note_spans_in_line(0, "seq | D4 . Eb4 |");
        let selected = note_at_or_near_col(notes, 99).unwrap();
        assert_eq!(selected.token, "Eb4");
    }

    #[test]
    fn ordered_note_span_bounds_sorts_by_position() {
        let notes = note_spans_in_line(0, "seq | D4 . Eb4 |");
        let (start, end) = ordered_note_span_bounds(&notes[1], &notes[0]);
        assert_eq!(start.token, "D4");
        assert_eq!(end.token, "Eb4");
    }
}
