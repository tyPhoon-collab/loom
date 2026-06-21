use super::input::PendingInput;
use super::keystroke::{lookup_key_action, KeyBinding, KeyStroke};
use super::source_text::char_to_byte_index;
use super::template_ops::template_call_spans_in_line;
use super::StudioApp;
use crossterm::event::{KeyCode, KeyEvent};
use miette::Result;
use ratatui_textarea::CursorMove;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RenameState {
    old_name: String,
    buffer: String,
}

#[derive(Clone, Copy, Debug)]
enum ChangeKeyAction {
    Cancel,
    RenameName,
}

const CHANGE_KEY_BINDINGS: &[KeyBinding<ChangeKeyAction>] = &[
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Esc),
        action: ChangeKeyAction::Cancel,
    },
    KeyBinding {
        stroke: KeyStroke::Char('n'),
        action: ChangeKeyAction::RenameName,
    },
];

impl RenameState {
    pub(super) fn prompt(&self) -> String {
        format!("Rename template @{} -> {}", self.old_name, self.buffer)
    }
}

impl StudioApp {
    pub(super) fn handle_change_key(&mut self, key: KeyEvent) -> Result<()> {
        let Some(action) = lookup_key_action(CHANGE_KEY_BINDINGS, &key) else {
            self.reject_pending_input(PendingInput::Change);
            return Ok(());
        };

        match action {
            ChangeKeyAction::Cancel => self.cancel_pending_input(PendingInput::Change),
            ChangeKeyAction::RenameName => self.begin_template_rename(),
        }
        Ok(())
    }

    pub(super) fn handle_rename_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.rename_state = None;
                self.status_message = "Rename cancelled".into();
            }
            KeyCode::Enter => self.accept_template_rename()?,
            KeyCode::Backspace => {
                if let Some(state) = self.rename_state.as_mut() {
                    state.buffer.pop();
                    self.status_message = state.prompt();
                }
            }
            KeyCode::Char(ch)
                if !super::keystroke::key_stroke_matches(
                    KeyStroke::CtrlChar(ch.to_ascii_lowercase()),
                    &key,
                ) =>
            {
                if let Some(state) = self.rename_state.as_mut() {
                    state.buffer.push(ch);
                    self.status_message = state.prompt();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn begin_template_rename(&mut self) {
        let Some(old_name) = self.template_rename_target_name() else {
            self.status_message =
                "Rename needs a local template definition or template call under cursor".into();
            return;
        };
        if old_name.contains('.') {
            self.status_message = "Library template calls cannot be renamed here".into();
            return;
        }

        self.rename_state = Some(RenameState {
            old_name: old_name.clone(),
            buffer: old_name,
        });
        if let Some(state) = &self.rename_state {
            self.status_message = state.prompt();
        }
    }

    fn accept_template_rename(&mut self) -> Result<()> {
        let Some(state) = self.rename_state.take() else {
            return Ok(());
        };
        let new_name = state.buffer.trim().to_string();
        match rename_template_in_lines(self.textarea.lines(), &state.old_name, &new_name) {
            Ok(lines) => {
                let cursor = self.textarea.cursor();
                self.push_source_undo();
                self.replace_source(lines.join("\n"));
                let row = cursor.0.min(self.textarea.lines().len().saturating_sub(1));
                let col = cursor.1.min(self.line_len(row));
                self.textarea
                    .move_cursor(CursorMove::Jump(row as u16, col as u16));
                self.dirty = true;
                self.compile_and_update_current_source()?;
                self.status_message =
                    format!("Renamed template @{} to @{}", state.old_name, new_name);
            }
            Err(message) => {
                self.status_message = message;
            }
        }
        Ok(())
    }

    fn template_rename_target_name(&self) -> Option<String> {
        let cursor = self.textarea.cursor();
        let line = self.textarea.lines().get(cursor.0)?;
        if let Some(name) = template_definition_name(line) {
            return Some(name.to_string());
        }
        template_call_spans_in_line(cursor.0, line)
            .into_iter()
            .find(|span| cursor.1 >= span.start_col && cursor.1 < span.end_col)
            .map(|call| call.template_name)
    }
}

fn rename_template_in_lines(
    lines: &[String],
    old_name: &str,
    new_name: &str,
) -> std::result::Result<Vec<String>, String> {
    validate_template_name(new_name)?;
    if old_name == new_name {
        return Err("Template name unchanged".to_string());
    }

    let definition_count = lines
        .iter()
        .filter(|line| template_definition_name(line).is_some_and(|name| name == old_name))
        .count();
    if definition_count == 0 {
        return Err(format!("Template @{} definition not found", old_name));
    }
    if definition_count > 1 {
        return Err(format!("Template @{} has multiple definitions", old_name));
    }
    if lines
        .iter()
        .any(|line| template_definition_name(line).is_some_and(|name| name == new_name))
    {
        return Err(format!("Template @{} already exists", new_name));
    }

    let mut renamed = lines.to_vec();
    for (row, line) in renamed.iter_mut().enumerate() {
        if template_definition_name(line).is_some_and(|name| name == old_name) {
            replace_template_definition_name(line, old_name, new_name);
        }

        let spans = template_call_spans_in_line(row, line);
        for span in spans.into_iter().rev() {
            if span.template_name == old_name {
                let start = char_to_byte_index(line, span.start_col + 2);
                let end = char_to_byte_index(line, span.start_col + 2 + old_name.chars().count());
                line.replace_range(start..end, new_name);
            }
        }
    }

    Ok(renamed)
}

fn template_definition_name(line: &str) -> Option<&str> {
    line.trim()
        .strip_prefix("# @")
        .map(str::trim)
        .filter(|name| !name.is_empty())
}

fn replace_template_definition_name(line: &mut String, old_name: &str, new_name: &str) {
    let Some(prefix_start) = line.find("# @") else {
        return;
    };
    let start_col = line[..prefix_start].chars().count() + 3;
    let start = char_to_byte_index(line, start_col);
    let end = char_to_byte_index(line, start_col + old_name.chars().count());
    line.replace_range(start..end, new_name);
}

fn validate_template_name(name: &str) -> std::result::Result<(), String> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_');
    if valid {
        Ok(())
    } else {
        Err("Template name must use ASCII letters, digits, `_`, and `-`".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{rename_template_in_lines, validate_template_name};

    #[test]
    fn rename_template_updates_definition_and_local_calls() {
        let lines = vec![
            "# @riff".to_string(),
            "seq | C4 D4 |".to_string(),
            "# Piano: 1".to_string(),
            "[@riff +12]*2 [@drums.riff]".to_string(),
        ];

        let renamed = rename_template_in_lines(&lines, "riff", "lead").unwrap();

        assert_eq!(
            renamed,
            vec![
                "# @lead",
                "seq | C4 D4 |",
                "# Piano: 1",
                "[@lead +12]*2 [@drums.riff]"
            ]
        );
    }

    #[test]
    fn rename_template_rejects_existing_name() {
        let lines = vec!["# @riff".to_string(), "# @lead".to_string()];

        assert_eq!(
            rename_template_in_lines(&lines, "riff", "lead").unwrap_err(),
            "Template @lead already exists"
        );
    }

    #[test]
    fn validate_template_name_uses_dsl_name_shape() {
        assert!(validate_template_name("riff_1-a").is_ok());
        assert!(validate_template_name("riff.one").is_err());
        assert!(validate_template_name("").is_err());
    }
}
