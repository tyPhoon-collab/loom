use super::template_ops::TemplateCallSpan;
use crate::dsl::note::Note;

#[derive(Clone, Debug)]
pub(super) enum StudioSelection {
    Unit {
        row: usize,
        start_col: usize,
        end_col: usize,
        token: String,
        kind: UnitKind,
    },
    UnitRange {
        anchor: UnitSpan,
        focus: UnitSpan,
    },
    Bar {
        span: BarSpan,
    },
    BarRange {
        anchor: BarSpan,
        focus: BarSpan,
    },
    TemplateCall {
        span: TemplateCallSpan,
    },
    TemplateCallRange {
        anchor: TemplateCallSpan,
        focus: TemplateCallSpan,
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
pub(super) struct UnitSpan {
    pub(super) row: usize,
    pub(super) start_col: usize,
    pub(super) end_col: usize,
    pub(super) token: String,
    pub(super) kind: UnitKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct GroupSpan {
    pub(super) start_col: usize,
    pub(super) end_col: usize,
    pub(super) selected_element: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum UnitKind {
    Pitch,
    SeqRest,
    SeqSustain,
    LaneNoteOn,
    LaneRest,
    LaneSustain,
    ModifierValue,
    ModifierEmpty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditableLineKind {
    Seq,
    Lane,
    Modifier(ModifierLineKind),
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModifierLineKind {
    Velocity,
    Pitch,
}

impl UnitKind {
    pub(super) fn is_pitch(self) -> bool {
        matches!(self, Self::Pitch)
    }

    pub(super) fn is_lane_body(self) -> bool {
        matches!(self, Self::LaneNoteOn | Self::LaneRest | Self::LaneSustain)
    }

    pub(super) fn is_seq_body(self) -> bool {
        matches!(self, Self::Pitch | Self::SeqRest | Self::SeqSustain)
    }

    pub(super) fn is_modifier(self) -> bool {
        matches!(self, Self::ModifierValue | Self::ModifierEmpty)
    }
}

pub(super) fn unit_spans_in_line(row: usize, line: &str) -> Vec<UnitSpan> {
    let Some(pipe_col) = line.chars().position(|ch| ch == '|') else {
        return Vec::new();
    };

    match editable_line_kind(line, pipe_col) {
        EditableLineKind::Seq => {
            let mut spans = Vec::new();
            scan_note_like_tokens(
                &mut spans,
                row,
                line,
                pipe_col + 1,
                line.chars().count(),
                true,
            );
            spans
        }
        EditableLineKind::Lane => {
            let mut spans = Vec::new();
            scan_note_like_tokens(&mut spans, row, line, 0, pipe_col, false);
            spans.extend(lane_body_token_spans_in_line(row, line));
            spans
        }
        EditableLineKind::Modifier(kind) => modifier_token_spans_in_line(row, line, pipe_col, kind),
        EditableLineKind::Other => Vec::new(),
    }
}

fn scan_note_like_tokens(
    spans: &mut Vec<UnitSpan>,
    row: usize,
    line: &str,
    scan_start_col: usize,
    scan_end_col: usize,
    is_seq: bool,
) {
    let mut token = String::new();
    let mut token_start = 0usize;

    for (col, ch) in line.chars().enumerate() {
        if col < scan_start_col || col >= scan_end_col {
            if !token.is_empty() {
                push_unit_span(spans, row, token_start, col, &token, is_seq);
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
            push_unit_span(spans, row, token_start, col, &token, is_seq);
            token.clear();
        }
    }

    if !token.is_empty() {
        push_unit_span(
            spans,
            row,
            token_start,
            line.chars().count(),
            &token,
            is_seq,
        );
    }
}

pub(super) fn lane_body_token_spans_in_line(row: usize, line: &str) -> Vec<UnitSpan> {
    let mut spans = Vec::new();
    if is_seq_line(line) || lane_head_token(line).is_none() {
        return spans;
    }

    let chars: Vec<char> = line.chars().collect();
    for bar in bar_spans_in_line(row, line) {
        for col in bar.start_col + 1..bar.end_col.saturating_sub(1) {
            let Some(ch) = chars.get(col).copied() else {
                continue;
            };
            let kind = match ch {
                '^' => UnitKind::LaneNoteOn,
                '.' => UnitKind::LaneRest,
                '-' => UnitKind::LaneSustain,
                _ => continue,
            };
            spans.push(UnitSpan {
                row,
                start_col: col,
                end_col: col + 1,
                token: ch.to_string(),
                kind,
            });
        }
    }
    spans
}

pub(super) fn lane_head_token(line: &str) -> Option<String> {
    let pipe_col = line.chars().position(|ch| ch == '|')?;
    let head: String = line.chars().take(pipe_col).collect();
    let head = head.trim();
    (!head.is_empty() && head != "seq").then(|| head.to_string())
}

pub(super) fn is_lane_body_token(line: &str, token: &UnitSpan) -> bool {
    if token.kind.is_lane_body() {
        return true;
    }

    !is_seq_line(line)
        && line
            .chars()
            .position(|ch| ch == '|')
            .is_some_and(|pipe_col| token.start_col > pipe_col)
}

pub(super) fn unit_at_or_near_col(notes: Vec<UnitSpan>, col: usize) -> Option<UnitSpan> {
    notes
        .iter()
        .find(|note| col >= note.start_col && col < note.end_col)
        .cloned()
        .or_else(|| notes.iter().find(|note| note.start_col >= col).cloned())
        .or_else(|| notes.into_iter().next_back())
}

pub(super) fn next_unit_after_position(
    notes: &[UnitSpan],
    row: usize,
    col: usize,
) -> Option<UnitSpan> {
    let current_index = notes
        .iter()
        .position(|note| note.row == row && col >= note.start_col && col < note.end_col);
    if let Some(index) = current_index {
        return notes.get(index + 1).cloned();
    }

    notes
        .iter()
        .find(|note| note.row > row || (note.row == row && note.start_col > col))
        .cloned()
}

pub(super) fn previous_unit_before_position(
    notes: &[UnitSpan],
    row: usize,
    col: usize,
) -> Option<UnitSpan> {
    let current_index = notes
        .iter()
        .position(|note| note.row == row && col >= note.start_col && col < note.end_col);
    if let Some(index) = current_index {
        return index
            .checked_sub(1)
            .and_then(|prev| notes.get(prev).cloned());
    }

    notes
        .iter()
        .rev()
        .find(|note| note.row < row || (note.row == row && note.start_col < col))
        .cloned()
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

pub(super) fn ordered_unit_span_bounds<'a>(
    left: &'a UnitSpan,
    right: &'a UnitSpan,
) -> (&'a UnitSpan, &'a UnitSpan) {
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

pub(super) fn delete_unit(line: &mut String, note: &UnitSpan) {
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
    matches!(
        line.chars().position(|ch| ch == '|'),
        Some(pipe_col) if editable_line_kind(line, pipe_col) == EditableLineKind::Seq
    )
}

pub(super) fn group_span_containing_col(line: &str, col: usize) -> Option<GroupSpan> {
    let chars: Vec<char> = line.chars().collect();
    let mut stack = Vec::new();
    let mut best: Option<(usize, usize)> = None;

    for (index, ch) in chars.iter().copied().enumerate() {
        match ch {
            '[' => stack.push(index),
            ']' => {
                let Some(start) = stack.pop() else {
                    continue;
                };
                if start < col
                    && col < index
                    && best.is_none_or(|(best_start, _)| start >= best_start)
                {
                    best = Some((start, index));
                }
            }
            _ => {}
        }
    }

    let (start_col, end_col) = best?;
    let selected_element = selected_group_element_text(&chars, start_col, end_col, col)?;
    Some(GroupSpan {
        start_col,
        end_col: end_col + 1,
        selected_element,
    })
}

fn selected_group_element_text(
    chars: &[char],
    start_col: usize,
    end_col: usize,
    selected_col: usize,
) -> Option<String> {
    first_level_group_elements(chars, start_col, end_col)
        .into_iter()
        .find(|(element_start, element_end, _)| {
            *element_start <= selected_col && selected_col < *element_end
        })
        .map(|(_, _, text)| text)
}

fn first_level_group_elements(
    chars: &[char],
    start_col: usize,
    end_col: usize,
) -> Vec<(usize, usize, String)> {
    let mut elements = Vec::new();
    let mut col = start_col + 1;
    while col < end_col {
        while col < end_col && chars.get(col).is_some_and(|ch| ch.is_whitespace()) {
            col += 1;
        }
        if col >= end_col {
            break;
        }

        let Some(ch) = chars.get(col).copied() else {
            break;
        };
        if ch == '[' {
            let Some(nested_end) = matching_group_end(chars, col) else {
                break;
            };
            elements.push((
                col,
                nested_end + 1,
                chars[col..=nested_end].iter().collect(),
            ));
            col = nested_end + 1;
            continue;
        }
        if is_group_token_char(ch) {
            let token_start = col;
            let mut token = String::new();
            while col < end_col {
                let Some(ch) = chars.get(col).copied() else {
                    break;
                };
                if is_group_token_char(ch) {
                    token.push(ch);
                    col += 1;
                } else {
                    break;
                }
            }
            elements.push((token_start, col, token));
            continue;
        }

        col += 1;
    }
    elements
}

fn matching_group_end(chars: &[char], start_col: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in chars.iter().copied().enumerate().skip(start_col) {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn push_unit_span(
    spans: &mut Vec<UnitSpan>,
    row: usize,
    start_col: usize,
    end_col: usize,
    token: &str,
    is_seq: bool,
) {
    let kind = match token {
        "." if is_seq => UnitKind::SeqRest,
        "-" if is_seq => UnitKind::SeqSustain,
        _ => {
            let Ok(note) = token.parse::<Note>() else {
                return;
            };
            if matches!(note, Note::Drum(_)) {
                return;
            }
            UnitKind::Pitch
        }
    };

    spans.push(UnitSpan {
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

fn is_group_token_char(ch: char) -> bool {
    is_selectable_token_char(ch) || is_modifier_token_char(ch) || ch == '^'
}

fn editable_line_kind(line: &str, pipe_col: usize) -> EditableLineKind {
    let head: String = line.chars().take(pipe_col).collect();
    match head.trim() {
        "seq" => EditableLineKind::Seq,
        "v" => EditableLineKind::Modifier(ModifierLineKind::Velocity),
        "p" => EditableLineKind::Modifier(ModifierLineKind::Pitch),
        head if !head.is_empty() => EditableLineKind::Lane,
        _ => EditableLineKind::Other,
    }
}

fn modifier_token_spans_in_line(
    row: usize,
    line: &str,
    pipe_col: usize,
    _kind: ModifierLineKind,
) -> Vec<UnitSpan> {
    let mut spans = Vec::new();
    let mut token = String::new();
    let mut token_start = 0usize;

    for (col, ch) in line.chars().enumerate() {
        if col <= pipe_col {
            continue;
        }

        if is_modifier_token_char(ch) {
            if token.is_empty() {
                token_start = col;
            }
            token.push(ch);
            continue;
        }

        if !token.is_empty() {
            push_modifier_span(&mut spans, row, token_start, col, &token);
            token.clear();
        }
    }

    if !token.is_empty() {
        push_modifier_span(&mut spans, row, token_start, line.chars().count(), &token);
    }

    spans
}

fn push_modifier_span(
    spans: &mut Vec<UnitSpan>,
    row: usize,
    start_col: usize,
    end_col: usize,
    token: &str,
) {
    let kind = if token == "." {
        UnitKind::ModifierEmpty
    } else {
        UnitKind::ModifierValue
    };

    spans.push(UnitSpan {
        row,
        start_col,
        end_col,
        token: token.to_string(),
        kind,
    });
}

fn is_modifier_token_char(ch: char) -> bool {
    ch.is_ascii_digit() || matches!(ch, '!' | '+' | '-' | '.' | ',')
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
        bar_at_or_near_col, bar_spans_in_line, delete_unit, group_span_containing_col,
        insert_at_col, next_unit_after_position, ordered_bar_span_bounds, ordered_unit_span_bounds,
        previous_unit_before_position, replace_char_range, unit_at_or_near_col, unit_spans_in_line,
        UnitKind,
    };

    #[test]
    fn token_spans_seq_body_includes_rest_and_sustain() {
        let spans = unit_spans_in_line(0, "seq | D4 . Eb4 - A#3 |");
        let tokens: Vec<_> = spans.iter().map(|span| span.token.as_str()).collect();
        let kinds: Vec<_> = spans.iter().map(|span| span.kind).collect();
        assert_eq!(tokens, vec!["D4", ".", "Eb4", "-", "A#3"]);
        assert_eq!(
            kinds,
            vec![
                UnitKind::Pitch,
                UnitKind::SeqRest,
                UnitKind::Pitch,
                UnitKind::SeqSustain,
                UnitKind::Pitch
            ]
        );
    }

    #[test]
    fn note_spans_note_head_and_lane_body() {
        let spans = unit_spans_in_line(0, "F4,C5 | ^ . |");
        let tokens: Vec<_> = spans.iter().map(|span| span.token.as_str()).collect();
        let kinds: Vec<_> = spans.iter().map(|span| span.kind).collect();
        assert_eq!(tokens, vec!["F4", "C5", "^", "."]);
        assert_eq!(
            kinds,
            vec![
                UnitKind::Pitch,
                UnitKind::Pitch,
                UnitKind::LaneNoteOn,
                UnitKind::LaneRest,
            ]
        );
    }

    #[test]
    fn note_spans_drum_lane_body_excludes_drum_head() {
        let spans = unit_spans_in_line(0, "kick | ^ . - |");
        let tokens: Vec<_> = spans.iter().map(|span| span.token.as_str()).collect();
        let kinds: Vec<_> = spans.iter().map(|span| span.kind).collect();
        assert_eq!(tokens, vec!["^", ".", "-"]);
        assert_eq!(
            kinds,
            vec![
                UnitKind::LaneNoteOn,
                UnitKind::LaneRest,
                UnitKind::LaneSustain,
            ]
        );
    }

    #[test]
    fn modifier_lines_are_reserved_for_future_scanner() {
        let spans = unit_spans_in_line(0, "v | !80 . 60 |");
        let tokens: Vec<_> = spans.iter().map(|span| span.token.as_str()).collect();
        let kinds: Vec<_> = spans.iter().map(|span| span.kind).collect();
        assert_eq!(tokens, vec!["!80", ".", "60"]);
        assert_eq!(
            kinds,
            vec![
                UnitKind::ModifierValue,
                UnitKind::ModifierEmpty,
                UnitKind::ModifierValue,
            ]
        );
    }

    #[test]
    fn modifier_group_values_are_selectable() {
        let spans = unit_spans_in_line(0, "p | [+2 . -1] 0 |");
        let tokens: Vec<_> = spans.iter().map(|span| span.token.as_str()).collect();
        assert_eq!(tokens, vec!["+2", ".", "-1", "0"]);
    }

    #[test]
    fn replace_selected_note_token() {
        let mut line = "seq | D4 . Eb4 |".to_string();
        let span = unit_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "Eb4")
            .unwrap();
        replace_char_range(&mut line, span.start_col, span.end_col, "E4");
        assert_eq!(line, "seq | D4 . E4 |");
    }

    #[test]
    fn delete_unit_removes_following_space() {
        let mut line = "seq | C4 D4 E4 |".to_string();
        let span = unit_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "D4")
            .unwrap();
        delete_unit(&mut line, &span);
        assert_eq!(line, "seq | C4 E4 |");
    }

    #[test]
    fn delete_unit_removes_adjacent_comma() {
        let mut line = "F4,C5,D5 | ^ |".to_string();
        let span = unit_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "C5")
            .unwrap();
        delete_unit(&mut line, &span);
        assert_eq!(line, "F4,D5 | ^ |");
    }

    #[test]
    fn delete_rest_token_removes_following_space() {
        let mut line = "seq | C4 . E4 |".to_string();
        let span = unit_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == ".")
            .unwrap();
        delete_unit(&mut line, &span);
        assert_eq!(line, "seq | C4 E4 |");
    }

    #[test]
    fn insert_at_col_inserts_text() {
        let mut line = "seq | C4 D4 E4 |".to_string();
        let span = unit_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "D4")
            .unwrap();
        insert_at_col(&mut line, span.end_col, " D4");
        assert_eq!(line, "seq | C4 D4 D4 E4 |");
    }

    #[test]
    fn note_select_uses_note_under_cursor() {
        let notes = unit_spans_in_line(0, "seq | D4 . Eb4 |");
        let selected = unit_at_or_near_col(notes, 6).unwrap();
        assert_eq!(selected.token, "D4");
    }

    #[test]
    fn note_select_falls_forward() {
        let notes = unit_spans_in_line(0, "seq | D4 . Eb4 |");
        let selected = unit_at_or_near_col(notes, 9).unwrap();
        assert_eq!(selected.token, ".");
    }

    #[test]
    fn note_select_falls_back_to_last_note() {
        let notes = unit_spans_in_line(0, "seq | D4 . Eb4 |");
        let selected = unit_at_or_near_col(notes, 99).unwrap();
        assert_eq!(selected.token, "Eb4");
    }

    #[test]
    fn ordered_unit_span_bounds_sorts_by_position() {
        let notes = unit_spans_in_line(0, "seq | D4 . Eb4 |");
        let (start, end) = ordered_unit_span_bounds(&notes[2], &notes[0]);
        assert_eq!(start.token, "D4");
        assert_eq!(end.token, "Eb4");
    }

    #[test]
    fn next_unit_skips_current_unit() {
        let notes = unit_spans_in_line(0, "seq | D4 . Eb4 |");
        let next = next_unit_after_position(&notes, 0, 6).unwrap();
        assert_eq!(next.token, ".");
    }

    #[test]
    fn previous_unit_returns_prior_unit() {
        let notes = unit_spans_in_line(0, "seq | D4 . Eb4 |");
        let previous = previous_unit_before_position(&notes, 0, 10).unwrap();
        assert_eq!(previous.token, ".");
    }

    #[test]
    fn next_unit_crosses_lines() {
        let mut notes = unit_spans_in_line(0, "seq | D4 . |");
        notes.extend(unit_spans_in_line(1, "kick | ^ . |"));
        let next = next_unit_after_position(&notes, 0, 9).unwrap();
        assert_eq!(next.row, 1);
        assert_eq!(next.token, "^");
    }

    #[test]
    fn group_span_containing_col_returns_selected_element() {
        let group = group_span_containing_col("seq | [C4 .] . |", 8).unwrap();
        assert_eq!(group.selected_element, "C4");
    }

    #[test]
    fn group_span_containing_col_uses_nearest_nested_group() {
        let line = "[^ [^ ^]]";
        let group = group_span_containing_col(line, 4).unwrap();
        assert_eq!(group.selected_element, "^");
        assert_eq!((group.start_col, group.end_col), (3, 8));
    }

    #[test]
    fn group_span_containing_col_keeps_nested_selected_element() {
        let line = "seq | [[C4 .] [. .]] . |";
        let group = group_span_containing_col(line, 9).unwrap();
        assert_eq!(group.selected_element, "C4");
        assert_eq!((group.start_col, group.end_col), (7, 13));
    }

    #[test]
    fn group_span_containing_col_picks_element_under_cursor() {
        let line = "seq | [C4 .] . |";
        let left = group_span_containing_col(line, 7).unwrap();
        let right = group_span_containing_col(line, 10).unwrap();
        assert_eq!(left.selected_element, "C4");
        assert_eq!(right.selected_element, ".");
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
