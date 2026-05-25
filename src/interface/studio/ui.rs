use super::input::PendingInput;
use super::selection::StudioSelection;
use super::settings::parse_track_header;
use super::{CompileStatus, StudioApp, StudioMode};
use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

const FOOTER_GLOBAL_HELP: &str = "Global: ? help  space play  r restart  w save  f format  q quit";
const FOOTER_NORMAL_HELP: &str =
    "Normal: a add  g goto  P preview  n/N note  o/O onset  v/V/b/B select";
const FOOTER_INSERT_HELP: &str = "Insert: type text  Esc normal  compile on exit";
const FOOTER_SELECT_HELP: &str = "Select: move hjkl  expand HJKL  n replace  x delete  d duplicate";
const FOOTER_NORMAL_TRACK_CONTEXT: &str =
    "Context: m mute  M solo  X clear-all  D s/l/t/h/T/b/v/p/m delete";
const FOOTER_NORMAL_TEMPLATE_CONTEXT: &str =
    "Context: g d goto-template  a m add-macro  +/-/[] transpose  </> repeat  {} time-scale  ,/. unit";
const FOOTER_NORMAL_EDIT_CONTEXT: &str =
    "Context: x delete-unit  s/S subdivide-shrink  ,/. unit  </> bar";
const FOOTER_SELECT_BAR_CONTEXT: &str = "Context: Enter loop-range  T template  +/- transpose";
const FOOTER_SELECT_LINE_CONTEXT: &str = "Context: +/- transpose  vertical selection";
const FOOTER_SELECT_TOKEN_CONTEXT: &str = "Context: s/S group edit  +/- transpose";
const FOOTER_SELECT_TEMPLATE_CONTEXT: &str =
    "Context: x delete  d duplicate  g d goto-template  +/-/[] transpose  </> repeat  {} time-scale";
const FOOTER_SELECT_EMPTY_CONTEXT: &str = "Context: Esc normal";

const OVERLAY_GLOBAL_LINES: &[&str] = &[
    "? toggle help overlay",
    "space play/pause  r restart  w save  f format  q quit  Q force quit",
];
const OVERLAY_NORMAL_LINES: &[&str] = &[
    "i insert  a add  g goto  P preview  n/N note  o/O onset",
    "v/V/b/B select  x delete-unit  s subdivide  S shrink",
    ",/. unit-nav  </> bar-nav  +/-/[] transpose",
    "m mute-track  M solo-track  X clear-all-track-flags  D s/l/t/h/T/b/v/p/m delete",
    "template call: g d goto-definition  a m then a/r/s adds arp/rev/strum  +/-/[] transpose  </> repeat  {} time-scale",
    "L toggle-loop  Ctrl-L clear-loop",
];
const OVERLAY_INSERT_LINES: &[&str] = &[
    "type to edit source directly",
    "Esc returns to Normal and recompiles",
    "Ctrl-U / Ctrl-R are handled by the textarea when available",
];
const OVERLAY_SELECT_LINES: &[&str] = &[
    "hjkl/arrows move focus  HJKL/Shift-arrows expand",
    "n note replace  o onset replace  x delete  d duplicate",
    "s subdivide  S shrink  +/-/[] transpose",
    "template call selection: x delete  d duplicate  g d goto-definition  +/-/[] transpose  </> repeat  {} time-scale",
    "Enter writes loop_range from bar selection  T extracts template",
];
const OVERLAY_CLOSE_LINES: &[&str] = &["Esc or ? closes this overlay"];

impl StudioApp {
    pub(super) fn ui(&mut self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Min(8),
                    Constraint::Length(7),
                    Constraint::Length(4),
                ]
                .as_ref(),
            )
            .split(f.area());

        let mode = match self.mode {
            StudioMode::Normal => "NORMAL",
            StudioMode::Insert => "INSERT",
            StudioMode::Select => "SELECT",
        };
        let dirty = if self.dirty { " *" } else { "" };
        let title = format!(
            "Score - {} [{}{} | {}]",
            self.path.display(),
            mode,
            dirty,
            self.cursor_label()
        );
        let block = Block::default().title(title).borders(Borders::ALL);
        let inner = block.inner(chunks[0]);
        f.render_widget(block, chunks[0]);
        f.render_widget(&self.textarea, inner);
        self.update_textarea_scroll_top(inner);
        self.render_selection_overlay(inner, f.buffer_mut());

        let beat_val = *self.current_beat.lock().unwrap();
        let compile_line = match &self.compile_status {
            CompileStatus::Ok {
                notes,
                controls,
                bpm,
            } => format!(
                "Compile: OK ({} notes, {} controls, {} BPM)",
                notes, controls, bpm
            ),
            CompileStatus::Error(message) => format!("Compile: {}", message),
        };
        let selection_line = match self.mode {
            StudioMode::Select => format!("Target: {}", self.selection_label()),
            _ => format!("Target: {}", self.cursor_label()),
        };
        let playback_state = if self.is_playing { "PLAYING" } else { "PAUSED" };
        let status = Paragraph::new(format!(
            "Device: {}\nPlayback: {}  Beat: {:.2}  BPM: {}  KeyOct: {}\n{}\n{}\nMessage: {}\n{}",
            self.midi_device_name,
            playback_state,
            beat_val,
            self.bpm,
            self.note_keyboard_octave,
            compile_line,
            selection_line,
            self.status_message,
            self.config_status
        ))
        .block(Block::default().title("Playback").borders(Borders::ALL))
        .style(match &self.compile_status {
            CompileStatus::Ok { .. } => Style::default().fg(Color::Green),
            CompileStatus::Error(_) => Style::default().fg(Color::Red),
        });
        f.render_widget(status, chunks[1]);

        let footer = Paragraph::new(self.footer_help_text())
            .block(Block::default().title("Help").borders(Borders::ALL))
            .wrap(Wrap { trim: true });
        f.render_widget(footer, chunks[2]);

        if self.show_help_overlay {
            self.render_help_overlay(f);
        }
    }

    fn footer_help_text(&self) -> String {
        let detail = match self.input_state.pending() {
            Some(pending) => format!("Pending: {}", pending.prompt(self.note_keyboard_octave)),
            None => match self.mode {
                StudioMode::Normal => {
                    format!("{}  {}", FOOTER_NORMAL_HELP, self.normal_context_help())
                }
                StudioMode::Insert => FOOTER_INSERT_HELP.to_string(),
                StudioMode::Select => {
                    format!("{}  {}", FOOTER_SELECT_HELP, self.select_context_help())
                }
            },
        };
        format!("{}\n{}", FOOTER_GLOBAL_HELP, detail)
    }

    fn normal_context_help(&self) -> &'static str {
        let cursor = self.textarea.cursor();
        if self
            .textarea
            .lines()
            .get(cursor.0)
            .and_then(|line| parse_track_header(line))
            .is_some()
        {
            FOOTER_NORMAL_TRACK_CONTEXT
        } else if self.current_template_call_at_cursor().is_some() {
            FOOTER_NORMAL_TEMPLATE_CONTEXT
        } else {
            FOOTER_NORMAL_EDIT_CONTEXT
        }
    }

    fn select_context_help(&self) -> &'static str {
        match self.selection {
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. }) => {
                FOOTER_SELECT_BAR_CONTEXT
            }
            Some(StudioSelection::LineRange { .. }) => FOOTER_SELECT_LINE_CONTEXT,
            Some(
                StudioSelection::TemplateCall { .. } | StudioSelection::TemplateCallRange { .. },
            ) => FOOTER_SELECT_TEMPLATE_CONTEXT,
            Some(StudioSelection::Unit { .. } | StudioSelection::UnitRange { .. }) => {
                FOOTER_SELECT_TOKEN_CONTEXT
            }
            None => FOOTER_SELECT_EMPTY_CONTEXT,
        }
    }

    fn render_help_overlay(&self, f: &mut ratatui::Frame) {
        let area = centered_rect(78, 75, f.area());
        let overlay = Paragraph::new(self.full_help_text())
            .block(
                Block::default()
                    .title("Keyboard Help")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(Clear, area);
        f.render_widget(overlay, area);
    }

    fn full_help_text(&self) -> String {
        let mut sections = Vec::new();
        push_help_section(&mut sections, "Global", OVERLAY_GLOBAL_LINES);

        if let Some(pending) = self.input_state.pending() {
            push_blank_line(&mut sections);
            sections.push("Pending".to_string());
            sections.push(format!("  {}", pending.help_text()));
            if matches!(pending, PendingInput::PreviewNote | PendingInput::Note(_)) {
                sections.push(format!("  current octave {}", self.note_keyboard_octave));
            }
            push_blank_line(&mut sections);
            push_help_section(&mut sections, "Close", OVERLAY_CLOSE_LINES);
            return sections.join("\n");
        }

        match self.mode {
            StudioMode::Normal => push_help_section(&mut sections, "Normal", OVERLAY_NORMAL_LINES),
            StudioMode::Insert => push_help_section(&mut sections, "Insert", OVERLAY_INSERT_LINES),
            StudioMode::Select => push_help_section(&mut sections, "Select", OVERLAY_SELECT_LINES),
        }

        push_blank_line(&mut sections);
        sections.push("Context".to_string());
        sections.push(format!(
            "  mode {} | target {}",
            match self.mode {
                StudioMode::Normal => "normal",
                StudioMode::Insert => "insert",
                StudioMode::Select => "select",
            },
            match self.mode {
                StudioMode::Select => self.selection_label(),
                _ => self.cursor_label(),
            }
        ));
        sections.extend(OVERLAY_CLOSE_LINES.iter().map(|line| format!("  {}", line)));

        sections.join("\n")
    }
}

fn push_blank_line(lines: &mut Vec<String>) {
    if !lines.is_empty() {
        lines.push(String::new());
    }
}

fn push_help_section(lines: &mut Vec<String>, title: &str, section_lines: &[&str]) {
    push_blank_line(lines);
    lines.push(title.to_string());
    lines.extend(section_lines.iter().map(|line| format!("  {}", line)));
}

fn centered_rect(horizontal_percent: u16, vertical_percent: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - vertical_percent) / 2),
        Constraint::Percentage(vertical_percent),
        Constraint::Percentage((100 - vertical_percent) / 2),
    ])
    .flex(Flex::Center)
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - horizontal_percent) / 2),
        Constraint::Percentage(horizontal_percent),
        Constraint::Percentage((100 - horizontal_percent) / 2),
    ])
    .flex(Flex::Center)
    .split(vertical[1])[1]
}
