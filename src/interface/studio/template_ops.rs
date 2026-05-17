use super::selection::{bar_spans_in_line, char_range, StudioSelection};
use super::settings::parse_track_header_channel;
use super::StudioApp;
use miette::Result;
use ratatui_textarea::CursorMove;

impl StudioApp {
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
}

struct TemplateExtraction {
    before_lines: Vec<String>,
    template_lines: Vec<String>,
    after_lines: Vec<String>,
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

fn next_template_name(
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

#[cfg(test)]
mod tests {
    use super::{extract_bar_rectangle, next_template_name, preferred_template_base_name};

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
}
