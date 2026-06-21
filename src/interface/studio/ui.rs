use super::input::PendingInput;
use super::preview::{PreviewControls, PreviewPanelState, PreviewTarget};
use super::preview_keyboard::preview_keyboard_deck_text;
use super::settings::parse_track_header;
use super::{CompileStatus, StudioApp, StudioMode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Flex, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

const FOOTER_GLOBAL_HELP: &str = "Global: ? help  space play  r restart  w save  f format  q quit";
const FOOTER_NORMAL_HELP: &str =
    "Normal: : command  n/N note  o/O onset  v/V/b/B select  x delete-unit  p paste  s/S grid";
const FOOTER_NORMAL_PREFIX_HELP: &str =
    "Prefix: a add  c change  d delete  g goto  Ctrl-o back  P preview-panel";
const FOOTER_INSERT_HELP: &str = "Insert: type text  Esc normal  compile on exit";
const FOOTER_COMPLETION_HELP: &str =
    "Completion: Ctrl-n/Down next  Ctrl-p/Up prev  Enter accept  Esc close";
const FOOTER_COMMAND_HELP: &str =
    "Command: type command  Enter run  Esc cancel  bpm 140 | loop on/off/clear | loop 0 4";
const FOOTER_SELECT_HELP: &str =
    "Select: : command  move hjkl  expand HJKL  n replace  d delete  y/p yank-paste";
const FOOTER_SELECT_DETAIL_HELP: &str =
    "Select+: x delete  s/S group  +/-/[] transpose  Enter loop-range  T template";
const FOOTER_PREVIEW_PANEL_HELP: &str =
    "Preview: note keys  1-5 target  [/] +/-1  { / } +/-10  Enter apply  r reset  Esc/P close";

const OVERLAY_GLOBAL_LINES: &[&str] = &[
    "? toggle help overlay",
    "space play/pause  r restart  w save  f format  q quit  Q force quit",
];
const OVERLAY_NORMAL_TRANSPORT_LINES: &[&str] =
    &["space play/pause  r restart  w save  f format  q quit  Q force quit"];
const OVERLAY_NORMAL_NAVIGATION_LINES: &[&str] = &["hjkl/arrows move  ,/. unit-nav  </> bar-nav"];
const OVERLAY_NORMAL_ENTRY_LINES: &[&str] = &[
    "i insert  P preview-panel  n/N note  o/O onset",
    "x delete-unit  p paste  s subdivide  S shrink  +/-/[] transpose",
];
const OVERLAY_NORMAL_SELECTION_LINES: &[&str] = &["v unit  V line  b bar  B line-bars"];
const OVERLAY_NORMAL_PREFIX_LINES: &[&str] =
    &["a add  c change  d delete  g goto  Ctrl-o previous file  : command"];
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
    "g d goto-definition/template-or-fragment  Ctrl-o previous file",
    "a m then a/r/s adds arp/rev/strum",
    "+/-/[] transpose  </> repeat  {} time-scale",
];
const OVERLAY_INSERT_LINES: &[&str] = &[
    "type to edit source directly",
    "Esc returns to Normal and recompiles",
    "Ctrl-U / Ctrl-R are handled by the textarea when available",
];
const OVERLAY_COMMAND_LINES: &[&str] = &[
    "Enter runs command  Esc cancels",
    "bpm 140",
    "loop on  loop off  loop clear  loop 0 4",
    "w  q  q!  wq  format  fmt",
];
const OVERLAY_SELECT_LINES: &[&str] = &[
    "hjkl/arrows move focus  HJKL/Shift-arrows expand",
    "n note replace  o onset replace  d/x delete  y yank  p paste",
    "s subdivide  S shrink  +/-/[] transpose",
    "template call selection: d/x delete  y yank  p paste  g d goto-definition  +/-/[] transpose  </> repeat  {} time-scale",
    "Enter writes loop_range from bar selection  T extracts template",
];
const OVERLAY_PREVIEW_PANEL_LINES: &[&str] = &[
    "play mapped note keys to audition the current track",
    "z/x octave down/up  . rest  - sustain",
    "1 pc  2 volume  3 pan  4 expression  5 mod",
    "[/] adjusts the selected target by -1/+1  { / } adjusts by -10/+10",
    "r resets unapplied preview changes to source values",
    "Enter applies unapplied preview changes to the current track",
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
            StudioMode::Command => "COMMAND",
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
        self.render_completion_popup(inner, f);

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
        if matches!(self.mode, StudioMode::Command) {
            return format!("Command: {}\n{}", self.command_buffer, FOOTER_COMMAND_HELP);
        }
        if let Some(rename) = &self.rename_state {
            return format!(
                "{}\n{} | Enter accept  Esc cancel",
                FOOTER_GLOBAL_HELP,
                rename.prompt()
            );
        }

        let detail = if self.preview_panel.open {
            FOOTER_PREVIEW_PANEL_HELP.to_string()
        } else if self.completion.is_some() {
            FOOTER_COMPLETION_HELP.to_string()
        } else {
            match self.input_state.pending() {
                Some(pending) => format!("Pending:\n{}", pending.prompt(self.note_keyboard_octave)),
                None => match self.mode {
                    StudioMode::Normal => {
                        format!("{}\n{}", FOOTER_NORMAL_HELP, FOOTER_NORMAL_PREFIX_HELP)
                    }
                    StudioMode::Insert => FOOTER_INSERT_HELP.to_string(),
                    StudioMode::Command => FOOTER_COMMAND_HELP.to_string(),
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
        let sections = Layout::vertical([
            Constraint::Length(4),
            Constraint::Length(10),
            Constraint::Length(1),
        ])
        .split(inner);
        f.render_widget(Clear, area);
        f.render_widget(block, area);
        f.render_widget(
            Paragraph::new(preview_control_panel_text(
                &self.preview_panel,
                self.note_keyboard_octave,
            ))
            .alignment(Alignment::Center),
            sections[0],
        );
        f.render_widget(
            Paragraph::new(preview_keyboard_deck_text(
                &self.note_keyboard.visual_layout(),
                &self.preview_panel.active_keys,
                self.note_keyboard_octave,
            ))
            .alignment(Alignment::Center),
            sections[1],
        );
        f.render_widget(
            Paragraph::new(preview_panel_brief_help()).alignment(Alignment::Center),
            sections[2],
        );
    }

    fn render_completion_popup(&self, score_area: Rect, f: &mut ratatui::Frame) {
        let Some(completion) = &self.completion else {
            return;
        };
        let candidates = completion.visible_candidates();
        if candidates.is_empty() || score_area.width < 8 || score_area.height < 3 {
            return;
        }

        let row = self.textarea.cursor().0 as i32 - self.textarea_viewport.top_row as i32;
        if row < 0 || row >= score_area.height as i32 {
            return;
        }
        let line_number_width = self.textarea.lines().len().max(1).to_string().len() as u16 + 2;
        let cursor_col = self.textarea.cursor().1 as i32 + line_number_width as i32
            - self.textarea_viewport.top_col as i32;
        let content_width = candidates
            .iter()
            .map(|candidate| candidate.insert_text.len() + candidate.label.len() + 4)
            .max()
            .unwrap_or(8)
            .min(48) as u16;
        let width = (content_width + 2).min(score_area.width);
        let height = (candidates.len() as u16 + 2).min(score_area.height);
        let x_offset = cursor_col
            .max(0)
            .min(score_area.width.saturating_sub(width) as i32) as u16;
        let below_y = row as u16 + 1;
        let y_offset = if below_y + height <= score_area.height {
            below_y
        } else {
            (row as u16).saturating_sub(height)
        };
        let area = Rect {
            x: score_area.x + x_offset,
            y: score_area.y + y_offset,
            width,
            height,
        };

        let lines = candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                let selected = index == completion.selected_index();
                let style = if selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan)
                } else {
                    Style::default().fg(Color::White).bg(Color::Rgb(28, 30, 34))
                };
                Line::from(vec![
                    Span::styled(if selected { "> " } else { "  " }, style),
                    Span::styled(candidate.insert_text.clone(), style),
                    Span::styled("  ", style),
                    Span::styled(candidate.label, style.fg(Color::DarkGray)),
                ])
            })
            .collect::<Vec<_>>();

        let title = if completion.query().is_empty() {
            completion.title().to_string()
        } else {
            format!("{}{} ", completion.title(), completion.query())
        };
        let popup = Paragraph::new(Text::from(lines))
            .block(Block::default().title(title).borders(Borders::ALL))
            .style(Style::default().bg(Color::Rgb(28, 30, 34)));
        f.render_widget(Clear, area);
        f.render_widget(popup, area);
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
                "  track {} | channel {} | target {} | source {} | preview {}",
                self.preview_panel.track_name,
                self.preview_panel.channel + 1,
                self.preview_panel.selected_target.label(),
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
            StudioMode::Command => {
                push_help_section(&mut sections, "Command", OVERLAY_COMMAND_LINES)
            }
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
                StudioMode::Command => "command",
            },
            match self.mode {
                StudioMode::Select => self.selection_label(),
                StudioMode::Command => self.command_buffer.clone(),
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

fn preview_control_panel_text(panel: &PreviewPanelState, octave: i32) -> Text<'static> {
    let mut status_spans = vec![
        lcd_span("TRACK", &panel.track_name, 30),
        Span::raw(" "),
        badge_span("CH", (panel.channel + 1).to_string(), BadgeTone::Dim),
    ];
    append_spaced(
        &mut status_spans,
        PreviewTarget::ALL.into_iter().map(|target| match target {
            PreviewTarget::Program => target_badge_span(
                target,
                panel.selected_target,
                panel.source_program,
                panel.effective_program(),
            ),
            _ => control_badge_span(target, panel.selected_target, &panel.controls),
        }),
    );
    append_spaced(
        &mut status_spans,
        [
            badge_span("OCT", octave.to_string(), BadgeTone::Normal),
            badge_span("VEL", panel.velocity.to_string(), BadgeTone::Normal),
        ],
    );

    let pads = [
        ("Z", "oct-", Color::Rgb(188, 246, 204)),
        ("X", "oct+", Color::Rgb(188, 246, 204)),
        ("1", "pc", Color::Rgb(181, 232, 255)),
        ("2", "vol", Color::Rgb(181, 232, 255)),
        ("3", "pan", Color::Rgb(181, 232, 255)),
        ("4", "exp", Color::Rgb(181, 232, 255)),
        ("5", "mod", Color::Rgb(181, 232, 255)),
        ("[", "-1", Color::Rgb(244, 196, 255)),
        ("]", "+1", Color::Rgb(244, 196, 255)),
        ("{", "-10", Color::Rgb(244, 196, 255)),
        ("}", "+10", Color::Rgb(244, 196, 255)),
    ];
    let mut pad_spans = Vec::new();
    append_spaced(
        &mut pad_spans,
        pads.into_iter()
            .map(|(key, label, color)| performance_pad_span(key, label, color)),
    );

    Text::from(vec![Line::from(status_spans), Line::from(pad_spans)])
}

fn append_spaced(spans: &mut Vec<Span<'static>>, items: impl IntoIterator<Item = Span<'static>>) {
    for item in items {
        if !spans.is_empty() {
            spans.push(Span::raw(" "));
        }
        spans.push(item);
    }
}

fn preview_panel_brief_help() -> Line<'static> {
    Line::from(vec![Span::styled(
        "1-5 select   [/] +/-1   {/} +/-10   Enter apply   r reset   Esc close",
        Style::default()
            .fg(Color::Rgb(124, 130, 148))
            .bg(Color::Rgb(28, 30, 34)),
    )])
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

#[derive(Clone, Copy)]
enum BadgeTone {
    Dim,
    Normal,
    Selected,
    Emphasized,
}

fn badge_span(label: &str, value: String, tone: BadgeTone) -> Span<'static> {
    let style = match tone {
        BadgeTone::Dim => Style::default()
            .fg(Color::Rgb(124, 130, 148))
            .bg(Color::Rgb(37, 39, 46)),
        BadgeTone::Normal => Style::default()
            .fg(Color::Rgb(188, 195, 216))
            .bg(Color::Rgb(44, 47, 54)),
        BadgeTone::Selected => Style::default()
            .fg(Color::Rgb(6, 24, 18))
            .bg(Color::Rgb(156, 244, 199)),
        BadgeTone::Emphasized => Style::default()
            .fg(Color::Rgb(32, 24, 0))
            .bg(Color::Rgb(255, 206, 82)),
    };
    Span::styled(format!(" {} {} ", label, value), style)
}

fn target_badge_span(
    target: PreviewTarget,
    selected_target: PreviewTarget,
    source_program: Option<u8>,
    preview_program: Option<u8>,
) -> Span<'static> {
    let source_matches_preview = source_program == preview_program;
    let value = match (source_program, preview_program) {
        (_, Some(preview)) => preview.to_string(),
        (Some(source), None) => source.to_string(),
        (None, None) => "device".to_string(),
    };
    badge_span(
        target.label(),
        value,
        if !source_matches_preview {
            BadgeTone::Emphasized
        } else if target == selected_target {
            BadgeTone::Selected
        } else {
            BadgeTone::Normal
        },
    )
}

fn control_badge_span(
    target: PreviewTarget,
    selected_target: PreviewTarget,
    controls: &PreviewControls,
) -> Span<'static> {
    let spec = target.control_spec().unwrap();
    let state = controls.get(target).unwrap_or_default();
    let value = state.effective_value(spec);
    let source = state.source.unwrap_or(spec.default_value);
    badge_span(
        target.label(),
        value.to_string(),
        if value != source {
            BadgeTone::Emphasized
        } else if target == selected_target {
            BadgeTone::Selected
        } else {
            BadgeTone::Normal
        },
    )
}

fn performance_pad_span(key: &str, label: &str, color: Color) -> Span<'static> {
    Span::styled(
        format!(" {:^5} {:<4} ", key, label),
        Style::default().fg(Color::Rgb(22, 24, 28)).bg(color),
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
    use super::super::preview::{PreviewPanelState, PreviewTarget};
    use super::preview_control_panel_text;

    #[test]
    fn preview_control_panel_emphasizes_changed_preview_program() {
        let mut panel = PreviewPanelState {
            track_name: "Lead".to_string(),
            channel: 2,
            source_program: Some(12),
            override_program: Some(42),
            selected_target: PreviewTarget::Volume,
            velocity: 96,
            ..PreviewPanelState::default()
        };
        panel.controls.volume.source = Some(90);
        panel.controls.volume.override_value = Some(100);
        let text = preview_control_panel_text(&panel, 4);
        let rendered = text
            .lines
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("TRACK Lead"));
        assert!(rendered.contains("PC 42"));
        assert!(rendered.contains("VOL 100"));
        assert!(rendered.contains("1   pc"));
        assert!(rendered.contains("2   vol"));
        assert!(rendered.contains("Z   oct-"));
        assert!(rendered.contains("[   -1"));
        assert!(rendered.contains("]   +1"));
    }
}
