use super::keystroke::{key_stroke_matches, KeyStroke};
use super::selection::StudioSelection;
use super::settings::{
    clear_loop_settings_frontmatter, loop_range_from_bounds, score_body_start_row,
    set_bpm_frontmatter, set_loop_enabled_frontmatter, set_loop_range_frontmatter,
};
use super::{StudioApp, StudioMode};
use crossterm::event::{KeyCode, KeyEvent};
use miette::Result;
use ratatui_textarea::CursorMove;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum StudioCommand {
    Bpm(u32),
    LoopOn,
    LoopOff,
    LoopClear,
    LoopRange { start: f64, end: f64 },
    Save,
    Quit,
    ForceQuit,
    SaveQuit,
    Format,
}

impl StudioApp {
    pub(super) fn handle_command_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.exit_command_mode("Command cancelled".to_string());
            }
            KeyCode::Enter => {
                let input = self.command_buffer.trim().to_string();
                let return_mode = self.command_return_mode.unwrap_or(StudioMode::Normal);
                self.mode = return_mode;
                self.command_return_mode = None;
                self.command_buffer.clear();
                self.execute_command_line(&input)?;
            }
            KeyCode::Backspace => {
                self.command_buffer.pop();
            }
            KeyCode::Char(ch)
                if !key_stroke_matches(KeyStroke::CtrlChar(ch.to_ascii_lowercase()), &key) =>
            {
                self.command_buffer.push(ch);
            }
            _ => {}
        }
        Ok(())
    }

    fn exit_command_mode(&mut self, message: String) {
        self.mode = self.command_return_mode.unwrap_or(StudioMode::Normal);
        self.command_return_mode = None;
        self.command_buffer.clear();
        self.status_message = message;
    }

    fn execute_command_line(&mut self, input: &str) -> Result<()> {
        let command = match parse_studio_command(input) {
            Ok(command) => command,
            Err(message) => {
                self.status_message = message;
                return Ok(());
            }
        };

        match command {
            StudioCommand::Bpm(bpm) => {
                let source = self.source();
                match set_bpm_frontmatter(&source, bpm) {
                    Ok(updated_source) => {
                        self.apply_command_source_update(
                            source,
                            updated_source,
                            format!("BPM: {}", bpm),
                        )?;
                    }
                    Err(message) => self.status_message = message,
                }
            }
            StudioCommand::LoopOn => {
                let source = self.source();
                match set_loop_enabled_frontmatter(&source, true) {
                    Ok(updated_source) => {
                        self.apply_command_source_update(
                            source,
                            updated_source,
                            "Loop: on".into(),
                        )?;
                    }
                    Err(message) => self.status_message = message,
                }
            }
            StudioCommand::LoopOff => {
                let source = self.source();
                match set_loop_enabled_frontmatter(&source, false) {
                    Ok(updated_source) => {
                        self.apply_command_source_update(
                            source,
                            updated_source,
                            "Loop: off".into(),
                        )?;
                    }
                    Err(message) => self.status_message = message,
                }
            }
            StudioCommand::LoopClear => match clear_loop_settings_frontmatter(&self.source()) {
                Ok(Some(updated_source)) => {
                    let source = self.source();
                    self.apply_command_source_update(
                        source,
                        updated_source,
                        "Loop cleared".into(),
                    )?;
                }
                Ok(None) => self.status_message = "No loop settings to clear".into(),
                Err(message) => self.status_message = message,
            },
            StudioCommand::LoopRange { start, end } => {
                let source = self.source();
                match loop_range_from_bounds(start, end).and_then(|range| {
                    set_loop_range_frontmatter(&source, &range).map(|source| (source, range))
                }) {
                    Ok((updated_source, range)) => {
                        self.apply_command_source_update(
                            source,
                            updated_source,
                            format!("Loop range: {}", range),
                        )?;
                    }
                    Err(message) => self.status_message = message,
                }
            }
            StudioCommand::Save => self.save()?,
            StudioCommand::Quit => {
                if self.dirty {
                    self.status_message = "Unsaved changes. Use w to save or q! to quit.".into();
                } else {
                    self.should_quit = true;
                }
            }
            StudioCommand::ForceQuit => {
                self.should_quit = true;
            }
            StudioCommand::SaveQuit => {
                self.save()?;
                self.should_quit = true;
            }
            StudioCommand::Format => self.format_current_source()?,
        }
        Ok(())
    }

    fn apply_command_source_update(
        &mut self,
        before_source: String,
        updated_source: String,
        status_message: String,
    ) -> Result<()> {
        let cursor = self.textarea.cursor();
        let next_row = shifted_score_row(cursor.0, &before_source, &updated_source);
        self.push_source_undo();
        self.replace_source(updated_source);
        let after_source = self.source();
        self.shift_selection_rows(&before_source, &after_source);
        self.textarea
            .move_cursor(CursorMove::Jump(next_row as u16, cursor.1 as u16));
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = status_message;
        Ok(())
    }

    fn shift_selection_rows(&mut self, before_source: &str, after_source: &str) {
        let Ok(before_start) = score_body_start_row(before_source) else {
            return;
        };
        let Ok(after_start) = score_body_start_row(after_source) else {
            return;
        };
        let delta = after_start as isize - before_start as isize;
        if delta == 0 {
            self.sync_selection_visual();
            return;
        }

        if let Some(selection) = self.selection.as_mut() {
            match selection {
                StudioSelection::Unit { row, .. } => shift_row(row, before_start, delta),
                StudioSelection::UnitRange { anchor, focus } => {
                    shift_row(&mut anchor.row, before_start, delta);
                    shift_row(&mut focus.row, before_start, delta);
                }
                StudioSelection::Bar { span } => shift_row(&mut span.row, before_start, delta),
                StudioSelection::BarRange { anchor, focus } => {
                    shift_row(&mut anchor.row, before_start, delta);
                    shift_row(&mut focus.row, before_start, delta);
                }
                StudioSelection::TemplateCall { span } => {
                    shift_row(&mut span.row, before_start, delta);
                }
                StudioSelection::TemplateCallRange { anchor, focus } => {
                    shift_row(&mut anchor.row, before_start, delta);
                    shift_row(&mut focus.row, before_start, delta);
                }
                StudioSelection::LineRange { anchor_row } => {
                    shift_row(anchor_row, before_start, delta);
                }
            }
        }
        self.sync_selection_visual();
    }
}

fn shifted_score_row(row: usize, before_source: &str, after_source: &str) -> usize {
    let Ok(before_start) = score_body_start_row(before_source) else {
        return row;
    };
    let Ok(after_start) = score_body_start_row(after_source) else {
        return row;
    };
    let delta = after_start as isize - before_start as isize;

    if row < before_start {
        row
    } else {
        row.saturating_add_signed(delta)
    }
}

fn shift_row(row: &mut usize, before_start: usize, delta: isize) {
    if *row >= before_start {
        *row = row.saturating_add_signed(delta);
    }
}

pub(super) fn parse_studio_command(input: &str) -> std::result::Result<StudioCommand, String> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let Some(command) = parts.first().copied() else {
        return Err("No command".to_string());
    };

    match command {
        "bpm" => parse_bpm_command(&parts),
        "loop" => parse_loop_command(&parts),
        "w" => expect_no_args(&parts, StudioCommand::Save),
        "q" => expect_no_args(&parts, StudioCommand::Quit),
        "q!" => expect_no_args(&parts, StudioCommand::ForceQuit),
        "wq" => expect_no_args(&parts, StudioCommand::SaveQuit),
        "format" | "fmt" => expect_no_args(&parts, StudioCommand::Format),
        _ => Err(format!("Unknown command: {}", command)),
    }
}

fn parse_bpm_command(parts: &[&str]) -> std::result::Result<StudioCommand, String> {
    if parts.len() != 2 {
        return Err("Usage: bpm 140".to_string());
    }
    let bpm = parts[1]
        .parse::<u32>()
        .map_err(|_| "BPM must be an integer 1..999".to_string())?;
    if !(1..=999).contains(&bpm) {
        return Err("BPM must be 1..999".to_string());
    }
    Ok(StudioCommand::Bpm(bpm))
}

fn parse_loop_command(parts: &[&str]) -> std::result::Result<StudioCommand, String> {
    match parts {
        ["loop", "on"] => Ok(StudioCommand::LoopOn),
        ["loop", "off"] => Ok(StudioCommand::LoopOff),
        ["loop", "clear"] => Ok(StudioCommand::LoopClear),
        ["loop", start, end] => {
            let start = parse_loop_bound(start)?;
            let end = parse_loop_bound(end)?;
            if end <= start {
                return Err("Loop range end must be greater than start".to_string());
            }
            Ok(StudioCommand::LoopRange { start, end })
        }
        _ => Err("Usage: loop on | loop off | loop clear | loop 0 4".to_string()),
    }
}

fn parse_loop_bound(value: &str) -> std::result::Result<f64, String> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| "Loop range bounds must be numbers".to_string())?;
    if !parsed.is_finite() || parsed < 0.0 {
        return Err("Loop range bounds must be non-negative numbers".to_string());
    }
    Ok(parsed)
}

fn expect_no_args(
    parts: &[&str],
    command: StudioCommand,
) -> std::result::Result<StudioCommand, String> {
    if parts.len() == 1 {
        Ok(command)
    } else {
        Err(format!("Command `{}` takes no arguments", parts[0]))
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_studio_command, StudioCommand};

    #[test]
    fn parses_bpm_command() {
        assert_eq!(
            parse_studio_command("bpm 140").unwrap(),
            StudioCommand::Bpm(140)
        );
    }

    #[test]
    fn rejects_invalid_bpm() {
        assert_eq!(
            parse_studio_command("bpm 0").unwrap_err(),
            "BPM must be 1..999"
        );
    }

    #[test]
    fn parses_loop_range_command() {
        assert_eq!(
            parse_studio_command("loop 0 4").unwrap(),
            StudioCommand::LoopRange {
                start: 0.0,
                end: 4.0
            }
        );
    }

    #[test]
    fn rejects_reversed_loop_range() {
        assert_eq!(
            parse_studio_command("loop 4 0").unwrap_err(),
            "Loop range end must be greater than start"
        );
    }

    #[test]
    fn parses_editor_commands() {
        assert_eq!(parse_studio_command("w").unwrap(), StudioCommand::Save);
        assert_eq!(
            parse_studio_command("q!").unwrap(),
            StudioCommand::ForceQuit
        );
        assert_eq!(parse_studio_command("fmt").unwrap(), StudioCommand::Format);
    }
}
