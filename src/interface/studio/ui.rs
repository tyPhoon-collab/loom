use super::input::PendingInput;
use super::preview_keyboard::preview_keyboard_deck_text;
use super::settings::parse_track_header;
use super::{CompileStatus, StudioApp, StudioMode};
use ratatui::{
    layout::{Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

const FOOTER_GLOBAL_HELP: &str = "Global: ? help  space play  r restart  w save  f format  q quit";
const FOOTER_NORMAL_HELP: &str =
    "Normal: n/N note  o/O onset  v/V/b/B select  x delete-unit  s/S grid";
const FOOTER_NORMAL_PREFIX_HELP: &str = "Prefix: a add  g goto  P preview-panel  D delete";
const FOOTER_INSERT_HELP: &str = "Insert: type text  Esc normal  compile on exit";
const FOOTER_SELECT_HELP: &str = "Select: move hjkl  expand HJKL  n replace  x delete  d duplicate";
const FOOTER_SELECT_DETAIL_HELP: &str =
    "Select+: s/S group  +/-/[] transpose  Enter loop-range  T template";
const FOOTER_PREVIEW_PANEL_HELP: &str =
    "Preview: note keys  z/x octave  [/] pc +/-1  { / } pc +/-10  r reset  Esc/P close";

const OVERLAY_GLOBAL_LINES: &[&str] = &[
    "? toggle help overlay",
    "space play/pause  r restart  w save  f format  q quit  Q force quit",
];
const OVERLAY_NORMAL_TRANSPORT_LINES: &[&str] =
    &["space play/pause  r restart  w save  f format  q quit  Q force quit"];
const OVERLAY_NORMAL_NAVIGATION_LINES: &[&str] = &["hjkl/arrows move  ,/. unit-nav  </> bar-nav"];
const OVERLAY_NORMAL_ENTRY_LINES: &[&str] = &[
    "i insert  P preview-panel  n/N note  o/O onset",
    "x delete-unit  s subdivide  S shrink  +/-/[] transpose",
];
const OVERLAY_NORMAL_SELECTION_LINES: &[&str] = &["v unit  V line  b bar  B line-bars"];
const OVERLAY_NORMAL_PREFIX_LINES: &[&str] =
    &["a add  D delete  g goto  L toggle-loop  Ctrl-L clear-loop"];
const OVERLAY_ADD_LINES: &[&str] = &[
    "s seq  l lane  t track  P piano-roll  h separator  T template  b bar",
    "d drums  v velocity  p pitch  i init  m macro  n note  . rest  - sustain",
];
const OVERLAY_DELETE_LINES: &[&str] = &[
    "s seq  l lane  t track  h separator  T template  b bar",
    "v velocity  p pitch  i init  m macro",
];
const OVERLAY_INIT_LINES: &[&str] = &[
    "p pc  b bank  c cc  n pan",
    "v volume  e expression  m mod  s sustain",
];
const OVERLAY_TEMPLATE_CALL_LINES: &[&str] = &[
    "g d goto-definition",
    "a m then a/r/s adds arp/rev/strum",
    "+/-/[] transpose  </> repeat  {} time-scale",
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
const OVERLAY_PREVIEW_PANEL_LINES: &[&str] = &[
    "play mapped note keys to audition the current track",
    "z/x octave down/up  . rest  - sustain",
    "[/] preview pc -1/+1  { / } preview pc -10/+10  r reset to source pc",
    "the panel shows an LCD-style control row, pad buttons, and a lit keyboard deck",
    "Esc or P closes the preview panel",
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
                    Constraint::Length(6),
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

        if self.preview_panel.open {
            self.render_preview_panel(f);
        }
    }

    fn footer_help_text(&self) -> String {
        let detail = if self.preview_panel.open {
            FOOTER_PREVIEW_PANEL_HELP.to_string()
        } else {
            match self.input_state.pending() {
                Some(pending) => format!("Pending:\n{}", pending.prompt(self.note_keyboard_octave)),
                None => match self.mode {
                    StudioMode::Normal => {
                        format!("{}\n{}", FOOTER_NORMAL_HELP, FOOTER_NORMAL_PREFIX_HELP)
                    }
                    StudioMode::Insert => FOOTER_INSERT_HELP.to_string(),
                    StudioMode::Select => {
                        format!("{}\n{}", FOOTER_SELECT_HELP, FOOTER_SELECT_DETAIL_HELP)
                    }
                },
            }
        };
        format!("{}\n{}", FOOTER_GLOBAL_HELP, detail)
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

    fn render_preview_panel(&self, f: &mut ratatui::Frame) {
        let area = centered_rect(98, 44, f.area());
        let chassis_style = Style::default().fg(Color::Gray).bg(Color::Rgb(28, 30, 34));
        let block = Block::default()
            .title(" Preview ")
            .borders(Borders::ALL)
            .style(chassis_style);
        let inner = block.inner(area);
        let sections =
            Layout::vertical([Constraint::Length(2), Constraint::Length(12)]).split(inner);
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        f.render_widget(
            Paragraph::new(preview_control_panel_text(
                &self.preview_panel.track_name,
                self.preview_panel.channel + 1,
                self.preview_panel.source_program,
                self.preview_panel.effective_program(),
                self.note_keyboard_octave,
                self.preview_panel.velocity,
            )),
            sections[0],
        );
        f.render_widget(
            Paragraph::new(preview_keyboard_deck_text(
                &self.note_keyboard.visual_layout(),
                &self.preview_panel.active_keys,
                self.note_keyboard_octave,
            )),
            sections[1],
        );
    }

    fn full_help_text(&self) -> String {
        let mut sections = Vec::new();
        push_help_section(&mut sections, "Global", OVERLAY_GLOBAL_LINES);

        if let Some(pending) = self.input_state.pending() {
            push_blank_line(&mut sections);
            sections.push("Pending".to_string());
            sections.extend(indent_lines(pending.help_text()));
            if matches!(pending, PendingInput::Note(_)) {
                sections.push(format!("  current octave {}", self.note_keyboard_octave));
            }
            push_blank_line(&mut sections);
            push_help_section(&mut sections, "Close", OVERLAY_CLOSE_LINES);
            return sections.join("\n");
        }

        if self.preview_panel.open {
            push_help_section(&mut sections, "Preview Panel", OVERLAY_PREVIEW_PANEL_LINES);
            push_blank_line(&mut sections);
            sections.push("Context".to_string());
            sections.push(format!(
                "  track {} | channel {} | source {} | preview {}",
                self.preview_panel.track_name,
                self.preview_panel.channel + 1,
                self.preview_panel
                    .source_program
                    .map(|program| format!("pc {}", program))
                    .unwrap_or_else(|| "unset".to_string()),
                self.preview_panel
                    .effective_program()
                    .map(|program| format!("pc {}", program))
                    .unwrap_or_else(|| "device current".to_string())
            ));
            sections.extend(OVERLAY_CLOSE_LINES.iter().map(|line| format!("  {}", line)));
            return sections.join("\n");
        }

        match self.mode {
            StudioMode::Normal => {
                push_help_section(&mut sections, "Transport", OVERLAY_NORMAL_TRANSPORT_LINES);
                push_help_section(&mut sections, "Navigation", OVERLAY_NORMAL_NAVIGATION_LINES);
                push_help_section(&mut sections, "Entry", OVERLAY_NORMAL_ENTRY_LINES);
                push_help_section(&mut sections, "Selection", OVERLAY_NORMAL_SELECTION_LINES);
                push_help_section(&mut sections, "Prefix", OVERLAY_NORMAL_PREFIX_LINES);
                push_help_section(&mut sections, "Add", OVERLAY_ADD_LINES);
                push_help_section(&mut sections, "Delete", OVERLAY_DELETE_LINES);
                push_help_section(&mut sections, "Init", OVERLAY_INIT_LINES);
                if self.current_template_call_at_cursor().is_some() {
                    push_help_section(&mut sections, "Template Call", OVERLAY_TEMPLATE_CALL_LINES);
                }
                if self
                    .textarea
                    .lines()
                    .get(self.textarea.cursor().0)
                    .and_then(|line| parse_track_header(line))
                    .is_some()
                {
                    push_help_section(
                        &mut sections,
                        "Track Header",
                        &["m mute-track  M solo-track  X clear-all-track-flags"],
                    );
                }
            }
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

fn indent_lines(text: &str) -> Vec<String> {
    text.lines().map(|line| format!("  {}", line)).collect()
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

fn preview_control_panel_text(
    track_name: &str,
    channel: u8,
    source_program: Option<u8>,
    preview_program: Option<u8>,
    octave: i32,
    velocity: u8,
) -> Text<'static> {
    let source_matches_preview = source_program == preview_program;
    Text::from(vec![
        Line::from(vec![
            lcd_span("TRACK", track_name, 18),
            Span::raw(" "),
            badge_span("CH", channel.to_string(), false),
            Span::raw(" "),
            badge_span(
                "SRC",
                source_program
                    .map(|program| format!("pc {}", program))
                    .unwrap_or_else(|| "unset".to_string()),
                false,
            ),
            Span::raw(" "),
            badge_span(
                "PREV",
                preview_program
                    .map(|program| format!("pc {}", program))
                    .unwrap_or_else(|| "device".to_string()),
                !source_matches_preview,
            ),
            Span::raw(" "),
            badge_span("OCT", octave.to_string(), false),
            Span::raw(" "),
            badge_span("VEL", velocity.to_string(), false),
        ]),
        Line::from(vec![
            pad_label_span("Z", "oct-"),
            Span::raw(" "),
            pad_label_span("X", "oct+"),
            Span::raw(" "),
            pad_label_span(".", "rest"),
            Span::raw(" "),
            pad_label_span("-", "sus"),
            Span::raw("   "),
            Span::styled(
                "Esc close  r reset  [ / ] pc-1/+1  { / } pc-10/+10",
                Style::default().fg(Color::Gray).bg(Color::Rgb(28, 30, 34)),
            ),
        ]),
    ])
}

fn lcd_span(label: &str, value: &str, width: usize) -> Span<'static> {
    let value = truncate_with_ellipsis(value, width);
    Span::styled(
        format!(" {} {:<width$} ", label, value, width = width),
        Style::default()
            .fg(Color::Rgb(156, 244, 199))
            .bg(Color::Rgb(8, 28, 19)),
    )
}

fn badge_span(label: &str, value: String, emphasized: bool) -> Span<'static> {
    let style = if emphasized {
        Style::default().fg(Color::Black).bg(Color::Yellow)
    } else {
        Style::default().fg(Color::Gray).bg(Color::Rgb(44, 47, 54))
    };
    Span::styled(format!(" {} {} ", label, value), style)
}

fn pad_label_span(key: &str, label: &str) -> Span<'static> {
    Span::styled(
        format!(" {}:{} ", key, label),
        Style::default()
            .fg(Color::DarkGray)
            .bg(Color::Rgb(28, 30, 34)),
    )
}

fn truncate_with_ellipsis(value: &str, width: usize) -> String {
    let count = value.chars().count();
    if count <= width {
        return value.to_string();
    }
    if width <= 1 {
        return "…".to_string();
    }
    let mut truncated: String = value.chars().take(width - 1).collect();
    truncated.push('…');
    truncated
}

#[cfg(test)]
mod tests {
    use super::preview_control_panel_text;

    #[test]
    fn preview_control_panel_emphasizes_changed_preview_program() {
        let text = preview_control_panel_text("Lead", 3, Some(12), Some(42), 4, 96);
        let rendered = text
            .lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("TRACK Lead"));
        assert!(rendered.contains("SRC pc 12"));
        assert!(rendered.contains("PREV pc 42"));
    }
}
