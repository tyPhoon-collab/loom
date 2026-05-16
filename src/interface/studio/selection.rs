use crate::dsl::note::Note;

#[derive(Clone, Debug)]
pub(super) enum StudioSelection {
    Note {
        row: usize,
        start_col: usize,
        end_col: usize,
        token: String,
        kind: SelectableTokenKind,
    },
    NoteRange {
        anchor: NoteTokenSpan,
        focus: NoteTokenSpan,
    },
    Bar {
        span: BarSpan,
    },
    BarRange {
        anchor: BarSpan,
        focus: BarSpan,
    },
    LineRange {
        anchor_row: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BarSpan {
    pub(super) row: usize,
    pub(super) start_col: usize,
    pub(super) end_col: usize,
    pub(super) index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NoteTokenSpan {
    pub(super) row: usize,
    pub(super) start_col: usize,
    pub(super) end_col: usize,
    pub(super) token: String,
    pub(super) kind: SelectableTokenKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SelectableTokenKind {
    Note,
    Rest,
    Sustain,
}

pub(super) fn note_spans_in_line(row: usize, line: &str) -> Vec<NoteTokenSpan> {
    let Some(pipe_col) = line.chars().position(|ch| ch == '|') else {
        return Vec::new();
    };

    let head: String = line.chars().take(pipe_col).collect();
    let is_seq = head.trim() == "seq";
    let scan_start_col = if is_seq { pipe_col + 1 } else { 0 };
    let scan_end_col = if is_seq {
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
                push_selectable_span(&mut spans, row, token_start, col, &token, is_seq);
                token.clear();
            }
            continue;
        }

        if is_selectable_token_char(ch) {
            if token.is_empty() {
                token_start = col;
            }
            token.push(ch);
        } else if !token.is_empty() {
            push_selectable_span(&mut spans, row, token_start, col, &token, is_seq);
            token.clear();
        }
    }

    if !token.is_empty() {
        push_selectable_span(
            &mut spans,
            row,
            token_start,
            line.chars().count(),
            &token,
            is_seq,
        );
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

pub(super) fn bar_spans_in_line(row: usize, line: &str) -> Vec<BarSpan> {
    let pipe_cols: Vec<usize> = line
        .chars()
        .enumerate()
        .filter_map(|(col, ch)| (ch == '|').then_some(col))
        .collect();

    pipe_cols
        .windows(2)
        .enumerate()
        .map(|(index, pair)| BarSpan {
            row,
            start_col: pair[0],
            end_col: pair[1] + 1,
            index,
        })
        .collect()
}

pub(super) fn bar_at_or_near_col(bars: Vec<BarSpan>, col: usize) -> Option<BarSpan> {
    bars.iter()
        .find(|bar| col >= bar.start_col && col < bar.end_col)
        .cloned()
        .or_else(|| bars.iter().find(|bar| bar.start_col >= col).cloned())
        .or_else(|| bars.into_iter().next_back())
}

pub(super) fn ordered_bar_span_bounds<'a>(
    left: &'a BarSpan,
    right: &'a BarSpan,
) -> (&'a BarSpan, &'a BarSpan) {
    if (left.row, left.start_col) <= (right.row, right.start_col) {
        (left, right)
    } else {
        (right, left)
    }
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

pub(super) fn char_range(line: &str, start_col: usize, end_col: usize) -> String {
    let start = char_to_byte_index(line, start_col);
    let end = char_to_byte_index(line, end_col);
    line[start..end].to_string()
}

pub(super) fn delete_note_token(line: &mut String, note: &NoteTokenSpan) {
    let chars: Vec<char> = line.chars().collect();
    let mut start_col = note.start_col;
    let mut end_col = note.end_col;

    if end_col < chars.len() && chars[end_col] == ',' {
        end_col += 1;
        while end_col < chars.len() && chars[end_col].is_whitespace() {
            end_col += 1;
        }
    } else if start_col > 0 && chars[start_col - 1] == ',' {
        start_col -= 1;
        while start_col > 0 && chars[start_col - 1].is_whitespace() {
            start_col -= 1;
        }
    } else if end_col < chars.len() && chars[end_col].is_whitespace() {
        while end_col < chars.len() && chars[end_col].is_whitespace() {
            end_col += 1;
        }
    } else {
        while start_col > 0 && chars[start_col - 1].is_whitespace() {
            start_col -= 1;
        }
    }

    replace_char_range(line, start_col, end_col, "");
}

pub(super) fn insert_at_col(line: &mut String, col: usize, text: &str) {
    let index = char_to_byte_index(line, col);
    line.insert_str(index, text);
}

pub(super) fn is_seq_line(line: &str) -> bool {
    let Some(pipe_col) = line.chars().position(|ch| ch == '|') else {
        return false;
    };
    let head: String = line.chars().take(pipe_col).collect();
    head.trim() == "seq"
}

fn push_selectable_span(
    spans: &mut Vec<NoteTokenSpan>,
    row: usize,
    start_col: usize,
    end_col: usize,
    token: &str,
    is_seq: bool,
) {
    let kind = match token {
        "." if is_seq => SelectableTokenKind::Rest,
        "-" if is_seq => SelectableTokenKind::Sustain,
        _ => {
            let Ok(note) = token.parse::<Note>() else {
                return;
            };
            if matches!(note, Note::Drum(_)) {
                return;
            }
            SelectableTokenKind::Note
        }
    };

    spans.push(NoteTokenSpan {
        row,
        start_col,
        end_col,
        token: token.to_string(),
        kind,
    });
}

pub(super) fn is_note_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '#' || ch == '-'
}

fn is_selectable_token_char(ch: char) -> bool {
    is_note_token_char(ch) || ch == '.'
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
        bar_at_or_near_col, bar_spans_in_line, delete_note_token, insert_at_col,
        note_at_or_near_col, note_spans_in_line, ordered_bar_span_bounds, ordered_note_span_bounds,
        replace_char_range, SelectableTokenKind,
    };

    #[test]
    fn token_spans_seq_body_includes_rest_and_sustain() {
        let spans = note_spans_in_line(0, "seq | D4 . Eb4 - A#3 |");
        let tokens: Vec<_> = spans.iter().map(|span| span.token.as_str()).collect();
        let kinds: Vec<_> = spans.iter().map(|span| span.kind).collect();
        assert_eq!(tokens, vec!["D4", ".", "Eb4", "-", "A#3"]);
        assert_eq!(
            kinds,
            vec![
                SelectableTokenKind::Note,
                SelectableTokenKind::Rest,
                SelectableTokenKind::Note,
                SelectableTokenKind::Sustain,
                SelectableTokenKind::Note
            ]
        );
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
    fn delete_note_token_removes_following_space() {
        let mut line = "seq | C4 D4 E4 |".to_string();
        let span = note_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "D4")
            .unwrap();
        delete_note_token(&mut line, &span);
        assert_eq!(line, "seq | C4 E4 |");
    }

    #[test]
    fn delete_note_token_removes_adjacent_comma() {
        let mut line = "F4,C5,D5 | ^ |".to_string();
        let span = note_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "C5")
            .unwrap();
        delete_note_token(&mut line, &span);
        assert_eq!(line, "F4,D5 | ^ |");
    }

    #[test]
    fn delete_rest_token_removes_following_space() {
        let mut line = "seq | C4 . E4 |".to_string();
        let span = note_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == ".")
            .unwrap();
        delete_note_token(&mut line, &span);
        assert_eq!(line, "seq | C4 E4 |");
    }

    #[test]
    fn insert_at_col_inserts_text() {
        let mut line = "seq | C4 D4 E4 |".to_string();
        let span = note_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "D4")
            .unwrap();
        insert_at_col(&mut line, span.end_col, " D4");
        assert_eq!(line, "seq | C4 D4 D4 E4 |");
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
        assert_eq!(selected.token, ".");
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
        let (start, end) = ordered_note_span_bounds(&notes[2], &notes[0]);
        assert_eq!(start.token, "D4");
        assert_eq!(end.token, "Eb4");
    }

    #[test]
    fn bar_spans_select_between_pipes_including_delimiters() {
        let bars = bar_spans_in_line(0, "C4 | ^ . | . ^ |");
        assert_eq!(bars.len(), 2);
        assert_eq!(
            (bars[0].start_col, bars[0].end_col, bars[0].index),
            (3, 10, 0)
        );
        assert_eq!(
            (bars[1].start_col, bars[1].end_col, bars[1].index),
            (9, 16, 1)
        );
    }

    #[test]
    fn bar_select_falls_forward_and_back() {
        let bars = bar_spans_in_line(0, "seq | C4 . | D4 . |");
        assert_eq!(bar_at_or_near_col(bars.clone(), 6).unwrap().index, 0);
        assert_eq!(bar_at_or_near_col(bars.clone(), 13).unwrap().index, 1);
        assert_eq!(bar_at_or_near_col(bars, 99).unwrap().index, 1);
    }

    #[test]
    fn ordered_bar_span_bounds_sorts_by_position() {
        let bars = bar_spans_in_line(0, "seq | C4 . | D4 . |");
        let (start, end) = ordered_bar_span_bounds(&bars[1], &bars[0]);
        assert_eq!(start.index, 0);
        assert_eq!(end.index, 1);
    }
}
