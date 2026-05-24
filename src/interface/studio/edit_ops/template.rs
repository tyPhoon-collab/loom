use crate::interface::studio::selection::replace_char_range;
use crate::interface::studio::StudioApp;
use miette::Result;

impl StudioApp {
    pub(crate) fn adjust_current_template_call_repeat(&mut self, delta: i32) -> Result<()> {
        let Some(call) = self.current_template_call_at_cursor() else {
            self.status_message = "No template call on this line".into();
            return Ok(());
        };
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(call.row) else {
            self.status_message = "Selected template call no longer exists".into();
            return Ok(());
        };
        let Some(replacement) = template_call_text_with_repeat_delta(&call.raw_text, delta) else {
            self.status_message = "Template call repeat needs a valid call".into();
            return Ok(());
        };
        replace_char_range(line, call.start_col, call.end_col, &replacement);
        self.apply_cursor_source_update(
            lines,
            (call.row, call.start_col),
            format!(
                "Adjusted template call @{} repeat by {:+}",
                call.template_name, delta
            ),
            None,
        )
    }

    pub(crate) fn adjust_selected_template_call_repeats(&mut self, delta: i32) -> Result<()> {
        let selected_indices = self.selected_template_call_indices();
        let mut selected = self.selected_template_call_spans();
        if selected.is_empty() {
            self.status_message = "No template call selected".into();
            return Ok(());
        }

        selected.sort_by(|left, right| {
            right
                .row
                .cmp(&left.row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });

        let mut lines = self.textarea.lines().to_vec();
        for span in &selected {
            let Some(line) = lines.get_mut(span.row) else {
                self.status_message = "Selected template call no longer exists".into();
                return Ok(());
            };
            let Some(replacement) = template_call_text_with_repeat_delta(&span.raw_text, delta)
            else {
                self.status_message = "Template call repeat needs a valid call".into();
                return Ok(());
            };
            replace_char_range(line, span.start_col, span.end_col, &replacement);
        }

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_template_call_selection_from_indices(&selected_indices);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Adjusted repeat on {} template call{} by {:+}",
            selected_indices.len(),
            if selected_indices.len() == 1 { "" } else { "s" },
            delta
        );
        Ok(())
    }

    pub(crate) fn adjust_current_template_call_time_scale(&mut self, delta: i32) -> Result<()> {
        let Some(call) = self.current_template_call_at_cursor() else {
            self.status_message = "No template call on this line".into();
            return Ok(());
        };
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(call.row) else {
            self.status_message = "Selected template call no longer exists".into();
            return Ok(());
        };
        let Some(replacement) = template_call_text_with_time_scale_delta(&call.raw_text, delta)
        else {
            self.status_message = "Template call time-scale needs a valid call".into();
            return Ok(());
        };
        replace_char_range(line, call.start_col, call.end_col, &replacement);
        self.apply_cursor_source_update(
            lines,
            (call.row, call.start_col),
            format!(
                "Adjusted template call @{} time-scale by {:+}",
                call.template_name, delta
            ),
            None,
        )
    }

    pub(crate) fn adjust_selected_template_call_time_scales(&mut self, delta: i32) -> Result<()> {
        let selected_indices = self.selected_template_call_indices();
        let mut selected = self.selected_template_call_spans();
        if selected.is_empty() {
            self.status_message = "No template call selected".into();
            return Ok(());
        }

        selected.sort_by(|left, right| {
            right
                .row
                .cmp(&left.row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });

        let mut lines = self.textarea.lines().to_vec();
        for span in &selected {
            let Some(line) = lines.get_mut(span.row) else {
                self.status_message = "Selected template call no longer exists".into();
                return Ok(());
            };
            let Some(replacement) = template_call_text_with_time_scale_delta(&span.raw_text, delta)
            else {
                self.status_message = "Template call time-scale needs a valid call".into();
                return Ok(());
            };
            replace_char_range(line, span.start_col, span.end_col, &replacement);
        }

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_template_call_selection_from_indices(&selected_indices);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Adjusted time-scale on {} template call{} by {:+}",
            selected_indices.len(),
            if selected_indices.len() == 1 { "" } else { "s" },
            delta
        );
        Ok(())
    }
}

pub(super) fn transposed_template_call_text(raw_text: &str, semitones: i32) -> Option<String> {
    let mut parsed = parse_template_call_editable(raw_text)?;
    parsed.transpose += semitones;
    if parsed.transpose == 0 {
        parsed.transpose = 0;
    }
    Some(parsed.to_text())
}

pub(super) fn template_call_text_with_repeat_delta(raw_text: &str, delta: i32) -> Option<String> {
    let mut parsed = parse_template_call_editable(raw_text)?;
    parsed.repeat = adjust_template_call_count(parsed.repeat, delta);
    Some(parsed.to_text())
}

pub(super) fn template_call_text_with_time_scale_delta(
    raw_text: &str,
    delta: i32,
) -> Option<String> {
    let mut parsed = parse_template_call_editable(raw_text)?;
    parsed.time_scale = adjust_template_call_count(parsed.time_scale, delta);
    Some(parsed.to_text())
}

pub(super) fn parse_template_call_transpose(input: &str) -> Option<i32> {
    let (sign, digits) = input.split_at(1);
    let value = digits.parse::<i32>().ok()?;
    match sign {
        "+" => Some(value),
        "-" => Some(-value),
        _ => None,
    }
}

pub(super) fn format_template_call_transpose(value: i32) -> String {
    if value >= 0 {
        format!("+{}", value)
    } else {
        value.to_string()
    }
}

pub(super) fn parse_template_call_repeat(input: &str) -> Option<u32> {
    input
        .strip_prefix('x')
        .and_then(|digits| digits.parse::<u32>().ok())
        .filter(|value| *value >= 1)
}

pub(super) fn parse_template_call_time_scale(input: &str) -> Option<u32> {
    input
        .strip_prefix('/')
        .and_then(|digits| digits.parse::<u32>().ok())
        .filter(|value| *value >= 1)
}

pub(super) fn format_template_call_repeat(value: u32) -> String {
    format!("x{}", value)
}

pub(super) fn format_template_call_time_scale(value: u32) -> String {
    format!("/{}", value)
}

fn adjust_template_call_count(current: u32, delta: i32) -> u32 {
    let updated = current as i32 + delta;
    updated.max(1) as u32
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedTemplateCallEditable {
    name: String,
    transpose: i32,
    repeat: u32,
    time_scale: u32,
    other_params: Vec<String>,
    suffix: String,
}

impl ParsedTemplateCallEditable {
    fn to_text(&self) -> String {
        let mut out = format!("[@{}", self.name);
        if self.transpose != 0 {
            out.push(' ');
            out.push_str(&format_template_call_transpose(self.transpose));
        }
        if self.repeat != 1 {
            out.push(' ');
            out.push_str(&format_template_call_repeat(self.repeat));
        }
        if self.time_scale != 1 {
            out.push(' ');
            out.push_str(&format_template_call_time_scale(self.time_scale));
        }
        for param in &self.other_params {
            out.push(' ');
            out.push_str(param);
        }
        out.push(']');
        out.push_str(&self.suffix);
        out
    }
}

fn parse_template_call_editable(raw_text: &str) -> Option<ParsedTemplateCallEditable> {
    let body = raw_text.strip_prefix("[@")?;
    let closing = body.find(']')?;
    let inside = &body[..closing];
    let suffix = &body[closing + 1..];

    let mut parts = inside.split_whitespace();
    let name = parts.next()?.to_string();
    let mut transpose = 0i32;
    let mut repeat = 1u32;
    let mut time_scale = 1u32;
    let mut other_params = Vec::new();

    for param in parts {
        if let Some(value) = parse_template_call_transpose(param) {
            transpose = value;
        } else if let Some(value) = parse_template_call_repeat(param) {
            repeat = value;
        } else if let Some(value) = parse_template_call_time_scale(param) {
            time_scale = value;
        } else {
            other_params.push(param.to_string());
        }
    }

    Some(ParsedTemplateCallEditable {
        name,
        transpose,
        repeat,
        time_scale,
        other_params,
        suffix: suffix.to_string(),
    })
}
