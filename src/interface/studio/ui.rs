use super::{CompileStatus, StudioApp, StudioMode};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

impl StudioApp {
    pub(super) fn ui(&mut self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Min(8),
                    Constraint::Length(7),
                    Constraint::Length(3),
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

        let help = match (self.mode, self.input_state.pending()) {
            (_, Some(pending)) => pending.help_text(),
            (StudioMode::Normal, None) => {
                "i ins | a add | g goto | n note | N note* | o onset | O onset* | m mute | M solo | X clear-all | D delete | x delete | ,/. token | </> bar | v note | b bar | +/- transpose | space play | w save"
            }
            (StudioMode::Insert, None) => "Esc normal | type to edit | Ctrl+U undo | Ctrl+R redo",
            (StudioMode::Select, None) => {
                "hjkl/arrows move | HJKL/Shift-arrows expand | n note | o onset | s/S subdivide-shrink | d duplicate | T template | x delete | Enter loop | Esc"
            }
        };
        let footer = Paragraph::new(help).block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    }
}
