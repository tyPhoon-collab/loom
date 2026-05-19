use super::input::PendingInput;
use super::selection::{bar_spans_in_line, char_range, StudioSelection};
use super::settings::parse_track_header_channel;
use super::StudioApp;
use crossterm::event::{KeyCode, KeyEvent};
use miette::Result;
use ratatui_textarea::CursorMove;

impl StudioApp {
    pub(super) fn current_template_call_at_cursor(&self) -> Option<TemplateCallSpan> {
        let cursor = self.textarea.cursor();
        self.textarea
            .lines()
            .get(cursor.0)
            .and_then(|line| template_call_at_or_near_col(cursor.0, line, cursor.1))
    }

    pub(super) fn handle_template_macro_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.status_message = PendingInput::TemplateMacro.cancel_message().into();
            }
            KeyCode::Char('a') => {
                self.insert_template_macro("arp")?;
            }
            KeyCode::Char('r') => {
                self.insert_template_macro("rev")?;
            }
            KeyCode::Char('s') => {
                self.insert_template_macro("strum")?;
            }
            _ => {
                self.status_message = PendingInput::TemplateMacro.unknown_message();
            }
        }
        Ok(())
    }

    pub(super) fn goto_current_template_definition(&mut self) -> Result<()> {
        let Some(call) = self.current_template_call_at_cursor() else {
            self.status_message = "Goto definition needs the cursor on a template call".into();
            return Ok(());
        };

        let Some(row) = self
            .textarea
            .lines()
            .iter()
            .enumerate()
            .find_map(|(row, line)| {
                line.trim()
                    .strip_prefix("# @")
                    .is_some_and(|name| name.trim() == call.template_name)
                    .then_some(row)
            })
        else {
            self.status_message = format!("Template @{} definition not found", call.template_name);
            return Ok(());
        };

        self.selection = None;
        self.textarea.cancel_selection();
        self.textarea.move_cursor(CursorMove::Jump(row as u16, 2));
        self.status_message = format!("Jumped to @{} definition", call.template_name);
        Ok(())
    }

    pub(super) fn extract_selected_bars_to_template(&mut self) -> Result<()> {
        let Some(StudioSelection::Bar { span: _ } | StudioSelection::BarRange { anchor: _, .. }) =
            self.selection.as_ref()
        else {
            self.status_message = "Template extraction applies to bar selection only".into();
            return Ok(());
        };

        let selected_bars = self.selected_bar_spans();
        if selected_bars.is_empty() {
            self.status_message = "Template extraction applies to bar selection only".into();
            return Ok(());
        }

        let (row_start, row_end, bar_start, bar_end) = match &self.selection {
            Some(StudioSelection::Bar { span }) => (span.row, span.row, span.index, span.index),
            Some(StudioSelection::BarRange { anchor, focus }) => {
                let ((row_start, row_end), (bar_start, bar_end)) =
                    self.selected_bar_rectangle_bounds(anchor, focus);
                (row_start, row_end, bar_start, bar_end)
            }
            _ => unreachable!(),
        };

        let mut lines = self.textarea.lines().to_vec();
        if !rows_all_have_bar_range(&lines, row_start, row_end, bar_start, bar_end) {
            self.status_message =
                "Template extraction needs matching bars on every selected line".into();
            return Ok(());
        }

        let Some(track_header_row) = common_track_header_row(&lines, row_start, row_end) else {
            self.status_message =
                "Template extraction currently supports one track at a time".into();
            return Ok(());
        };

        let template_name = next_template_name(&lines, track_header_row, row_start, row_end);
        let extraction = extract_bar_rectangle(&lines, row_start, row_end, bar_start, bar_end)?;

        let replacement_start = row_start;
        let replacement_end = row_end;
        let call_line = format!("[@{}]", template_name);
        let call_row = replacement_start + extraction.before_lines.len();

        let mut replacement = extraction.before_lines;
        replacement.push(call_line);
        replacement.extend(extraction.after_lines);
        lines.splice(replacement_start..=replacement_end, replacement);

        let insertion_index = lines.len();
        if insertion_index > 0 && !lines.last().is_some_and(|line| line.trim().is_empty()) {
            lines.push(String::new());
        }
        lines.push(format!("# @{}", template_name));
        lines.push(String::new());
        lines.extend(extraction.template_lines);

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.textarea
            .move_cursor(CursorMove::Jump(call_row as u16, 0));
        self.selection = None;
        self.textarea.cancel_selection();
        self.mode = super::StudioMode::Normal;
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Extracted {} bar{} to @{} from track line {}",
            selected_bars.len(),
            if selected_bars.len() == 1 { "" } else { "s" },
            template_name,
            track_header_row + 1
        );
        Ok(())
    }

    fn insert_template_macro(&mut self, macro_name: &str) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(cursor.0) else {
            self.status_message = "No current line".into();
            return Ok(());
        };
        let Some(call) = template_call_at_or_near_col(cursor.0, line, cursor.1) else {
            self.status_message = "Template macro add needs the cursor on a template call".into();
            return Ok(());
        };

        insert_template_macro_before_closing_bracket(line, &call, macro_name);
        self.apply_cursor_source_update(
            lines,
            (cursor.0, call.start_col),
            format!("Added {} macro to @{}", macro_name, call.template_name),
            None,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TemplateCallSpan {
    pub(super) row: usize,
    pub(super) start_col: usize,
    pub(super) end_col: usize,
    pub(super) template_name: String,
    pub(super) raw_text: String,
    closing_bracket_col: usize,
}

struct TemplateExtraction {
    before_lines: Vec<String>,
    template_lines: Vec<String>,
    after_lines: Vec<String>,
}

pub(super) fn template_call_spans_in_line(row: usize, line: &str) -> Vec<TemplateCallSpan> {
    let chars: Vec<char> = line.chars().collect();
    let mut spans = Vec::new();
    let mut col = 0usize;

    while col + 1 < chars.len() {
        if chars[col] != '[' || chars[col + 1] != '@' {
            col += 1;
            continue;
        }

        let Some(closing_bracket_col) = chars
            .iter()
            .enumerate()
            .skip(col + 2)
            .find_map(|(index, ch)| (*ch == ']').then_some(index))
        else {
            break;
        };

        let mut end_col = closing_bracket_col + 1;
        if chars.get(end_col).is_some_and(|ch| *ch == '*') {
            end_col += 1;
            while chars.get(end_col).is_some_and(|ch| ch.is_ascii_digit()) {
                end_col += 1;
            }
        }

        let body: String = chars[col + 2..closing_bracket_col].iter().collect();
        let template_name = body
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string();
        let raw_text: String = chars[col..end_col].iter().collect();
        spans.push(TemplateCallSpan {
            row,
            start_col: col,
            end_col,
            template_name,
            raw_text,
            closing_bracket_col,
        });
        col = end_col;
    }

    spans
}

pub(super) fn template_call_at_or_near_col(
    row: usize,
    line: &str,
    col: usize,
) -> Option<TemplateCallSpan> {
    let spans = template_call_spans_in_line(row, line);
    spans
        .iter()
        .find(|span| col >= span.start_col && col < span.end_col)
        .cloned()
        .or_else(|| spans.iter().find(|span| span.start_col >= col).cloned())
        .or_else(|| spans.into_iter().next_back())
}

fn insert_template_macro_before_closing_bracket(
    line: &mut String,
    call: &TemplateCallSpan,
    macro_name: &str,
) {
    let insert_col = call.closing_bracket_col;
    let insert_at = char_to_byte_index(line, insert_col);
    line.insert_str(insert_at, &format!(" {}", macro_name));
}

fn extract_bar_rectangle(
    lines: &[String],
    row_start: usize,
    row_end: usize,
    bar_start: usize,
    bar_end: usize,
) -> Result<TemplateExtraction> {
    let mut before_lines = Vec::new();
    let mut template_lines = Vec::new();
    let mut after_lines = Vec::new();

    for (row, line) in lines.iter().enumerate().take(row_end + 1).skip(row_start) {
        let all_bars = bar_spans_in_line(row, line);
        let first_bar = &all_bars[bar_start];
        let last_bar = &all_bars[bar_end];
        let line_head = line_head_prefix(line);

        if bar_start > 0 {
            let body = char_range(line, all_bars[0].start_col, first_bar.start_col + 1);
            before_lines.push(format_line_slice(&line_head, &body));
        }

        let selected_body = char_range(line, first_bar.start_col, last_bar.end_col);
        template_lines.push(format_line_slice(&line_head, &selected_body));

        if bar_end + 1 < all_bars.len() {
            let body = char_range(
                line,
                last_bar.end_col.saturating_sub(1),
                all_bars
                    .last()
                    .map(|bar| bar.end_col)
                    .unwrap_or(last_bar.end_col),
            );
            after_lines.push(format_line_slice(&line_head, &body));
        }
    }

    Ok(TemplateExtraction {
        before_lines,
        template_lines,
        after_lines,
    })
}

fn format_line_slice(head: &str, body: &str) -> String {
    format!("{} {}", head.trim_end(), body.trim_start())
}

fn line_head_prefix(line: &str) -> String {
    let pipe_col = line.chars().position(|ch| ch == '|').unwrap_or(line.len());
    line.chars().take(pipe_col).collect::<String>()
}

fn rows_all_have_bar_range(
    lines: &[String],
    row_start: usize,
    row_end: usize,
    bar_start: usize,
    bar_end: usize,
) -> bool {
    (row_start..=row_end).all(|row| {
        let bars = bar_spans_in_line(row, &lines[row]);
        bars.len() > bar_end && bar_start <= bar_end
    })
}

fn common_track_header_row(lines: &[String], row_start: usize, row_end: usize) -> Option<usize> {
    let mut header_row = None;
    for row in row_start..=row_end {
        let found = (0..=row)
            .rev()
            .find(|&index| parse_track_header_channel(&lines[index]).is_some())?;
        match header_row {
            Some(existing) if existing != found => return None,
            None => header_row = Some(found),
            _ => {}
        }
    }
    header_row
}

pub(super) fn next_template_name(
    lines: &[String],
    track_header_row: usize,
    row_start: usize,
    row_end: usize,
) -> String {
    let existing: std::collections::HashSet<String> = lines
        .iter()
        .filter_map(|line| line.trim().strip_prefix("# @"))
        .map(|name| name.trim().to_string())
        .collect();

    let preferred = preferred_template_base_name(lines, track_header_row, row_start, row_end);
    if !existing.contains(&preferred) {
        return preferred;
    }

    let mut index = 1usize;
    loop {
        let candidate = format!("{}{}", preferred, index);
        if !existing.contains(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

fn preferred_template_base_name(
    lines: &[String],
    track_header_row: usize,
    row_start: usize,
    row_end: usize,
) -> String {
    if row_start == row_end {
        let line_head = line_head_prefix(&lines[row_start]);
        let head = line_head.trim();
        if head == "seq" {
            return "riff".to_string();
        }
        if !head.is_empty() && head != "seq" {
            let slug = slugify_template_name(head);
            if !slug.is_empty() {
                return slug;
            }
        }
    }

    track_template_base_name(&lines[track_header_row]).unwrap_or_else(|| "template".to_string())
}

fn track_template_base_name(track_header: &str) -> Option<String> {
    let trimmed = track_header.trim();
    if trimmed.starts_with("##") || !trimmed.starts_with('#') {
        return None;
    }
    let body = trimmed.strip_prefix('#')?.trim();
    let (name, _) = body.split_once(':')?;
    let slug = slugify_template_name(name);
    (!slug.is_empty()).then_some(slug)
}

fn slugify_template_name(input: &str) -> String {
    let mut slug = String::new();
    let mut last_was_sep = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_sep = false;
        } else if !last_was_sep && !slug.is_empty() {
            last_was_sep = true;
        }
    }
    slug
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
        extract_bar_rectangle, insert_template_macro_before_closing_bracket, next_template_name,
        preferred_template_base_name, template_call_at_or_near_col, template_call_spans_in_line,
    };

    #[test]
    fn extracts_selected_bar_rectangle_into_template_slices() {
        let lines = vec![
            "kick | . ^ | - . | ^ . |".to_string(),
            "snare | . . | ^ . | . . |".to_string(),
        ];

        let extraction = extract_bar_rectangle(&lines, 0, 1, 1, 1).unwrap();
        assert_eq!(
            extraction.before_lines,
            vec!["kick | . ^ |", "snare | . . |"]
        );
        assert_eq!(
            extraction.template_lines,
            vec!["kick | - . |", "snare | ^ . |"]
        );
        assert_eq!(
            extraction.after_lines,
            vec!["kick | ^ . |", "snare | . . |"]
        );
    }

    #[test]
    fn picks_next_template_name() {
        let lines = vec![
            "# Piano: 1".to_string(),
            "# @piano".to_string(),
            "# @piano1".to_string(),
            "# @riff".to_string(),
        ];
        assert_eq!(next_template_name(&lines, 0, 1, 2), "piano2");
    }

    #[test]
    fn prefers_lane_name_for_single_line_template() {
        let lines = vec!["# Drums: 10".to_string(), "kick | ^ . |".to_string()];
        assert_eq!(preferred_template_base_name(&lines, 0, 1, 1), "kick");
    }

    #[test]
    fn prefers_riff_for_single_seq_line_template() {
        let lines = vec!["# Piano: 1".to_string(), "seq | C4 E4 |".to_string()];
        assert_eq!(preferred_template_base_name(&lines, 0, 1, 1), "riff");
    }

    #[test]
    fn template_call_span_includes_repeat_suffix() {
        let spans = template_call_spans_in_line(0, "[@riff +12]*2 [@bass]");
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].template_name, "riff");
        assert_eq!(spans[0].raw_text, "[@riff +12]*2");
        assert_eq!(spans[1].template_name, "bass");
    }

    #[test]
    fn template_call_at_or_near_col_prefers_span_under_cursor() {
        let span = template_call_at_or_near_col(0, "[@riff] [@bass]", 2).unwrap();
        assert_eq!(span.template_name, "riff");
    }

    #[test]
    fn template_macro_insert_happens_before_closing_bracket() {
        let mut line = "[@riff]*2".to_string();
        let call = template_call_at_or_near_col(0, &line, 1).unwrap();
        insert_template_macro_before_closing_bracket(&mut line, &call, "arp");
        assert_eq!(line, "[@riff arp]*2");
    }
}
