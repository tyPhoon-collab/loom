use crate::compiler;
use crate::dsl::note::Note;
use crate::dsl::{formatter, parser};
use crate::event::Event;
use crate::live_player::LivePlayer;
use crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind};
use miette::{IntoDiagnostic, Result};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_textarea::CursorMove;
use ratatui_textarea::TextArea;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StudioMode {
    Normal,
    Insert,
    Select,
}

pub struct StudioApp {
    should_quit: bool,
    path: PathBuf,
    mode: StudioMode,
    status_message: String,
    dirty: bool,
    is_playing: bool,
    bpm: u32,
    midi_device_name: String,
    config_status: String,
    current_beat: Arc<Mutex<f64>>,
    textarea: TextArea<'static>,
    source_undo_stack: Vec<String>,
    player: LivePlayer,
}

impl StudioApp {
    pub fn new(path: PathBuf, port_index: usize, config_status: String) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .map_err(|e| miette::miette!("Failed to read {}: {}", path.display(), e))?;
        let mut textarea = if content.is_empty() {
            TextArea::default()
        } else {
            TextArea::from(content.lines())
        };
        textarea.set_line_number_style(Style::default().fg(Color::DarkGray));
        textarea.set_cursor_line_style(Style::default().add_modifier(Modifier::REVERSED));
        textarea.set_selection_style(Style::default().bg(Color::Blue));

        let current_beat = Arc::new(Mutex::new(0.0));
        let player = LivePlayer::new(port_index, Arc::clone(&current_beat))?;
        let midi_device_name = midi_device_name(port_index);

        let mut app = Self {
            should_quit: false,
            path,
            mode: StudioMode::Normal,
            status_message: "Ready".to_string(),
            dirty: false,
            is_playing: false,
            bpm: 120,
            midi_device_name,
            config_status,
            current_beat,
            textarea,
            source_undo_stack: Vec::new(),
            player,
        };
        app.compile_and_update_current_source()?;
        Ok(app)
    }

    pub fn run<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut ratatui::Terminal<B>,
    ) -> Result<()> {
        self.player.play();
        self.is_playing = true;

        loop {
            terminal
                .draw(|f| self.ui(f))
                .map_err(|e| miette::miette!("Draw error: {:?}", e))?;

            if event::poll(Duration::from_millis(33)).into_diagnostic()? {
                if let event::Event::Key(key) = event::read().into_diagnostic()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key)?;
                    }
                }
            } else {
                self.handle_tick(Event::Tick);
            }

            if self.should_quit {
                break;
            }
        }
        self.player.stop();
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.mode {
            StudioMode::Normal => self.handle_normal_key(key),
            StudioMode::Insert => self.handle_insert_key(key),
            StudioMode::Select => self.handle_select_key(key),
        }
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('q') => {
                if self.dirty {
                    self.status_message = "Unsaved changes. Press w to save or Q to quit.".into();
                } else {
                    self.should_quit = true;
                }
            }
            KeyCode::Char('Q') => {
                self.should_quit = true;
            }
            KeyCode::Char('i') => {
                self.mode = StudioMode::Insert;
                self.status_message = "Insert mode".into();
            }
            KeyCode::Char('v') => {
                self.mode = StudioMode::Select;
                self.textarea.start_selection();
                self.status_message = "Select mode".into();
            }
            KeyCode::Char('w') => {
                self.save()?;
            }
            KeyCode::Char('f') => {
                self.format_current_source()?;
            }
            KeyCode::Char(' ') => {
                if self.is_playing {
                    self.player.pause();
                    self.is_playing = false;
                    self.status_message = "Paused".into();
                } else {
                    self.player.play();
                    self.is_playing = true;
                    self.status_message = "Playing".into();
                }
            }
            KeyCode::Char('r') => {
                self.player.restart();
                if !self.is_playing {
                    self.player.play();
                    self.is_playing = true;
                }
                self.status_message = "Restarted from beginning".into();
            }
            KeyCode::Char('u') => {
                if self.textarea.undo() {
                    self.dirty = true;
                    self.compile_and_update_current_source()?;
                } else if let Some(source) = self.source_undo_stack.pop() {
                    self.replace_source(source);
                    self.dirty = true;
                    self.compile_and_update_current_source()?;
                    self.status_message = "Undid transform".into();
                }
            }
            KeyCode::Char('R') => {
                if self.textarea.redo() {
                    self.dirty = true;
                    self.compile_and_update_current_source()?;
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.apply_transpose(1);
            }
            KeyCode::Char('>') => {
                self.apply_transpose(1);
            }
            KeyCode::Char('-') => {
                self.apply_transpose(-1);
            }
            KeyCode::Char('<') => {
                self.apply_transpose(-1);
            }
            KeyCode::Char(']') => {
                self.apply_transpose(12);
            }
            KeyCode::Char('[') => {
                self.apply_transpose(-12);
            }
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                self.textarea.input(key);
            }
            KeyCode::Char('j') => self.textarea.move_cursor(CursorMove::Down),
            KeyCode::Char('k') => self.textarea.move_cursor(CursorMove::Up),
            KeyCode::Char('h') => self.textarea.move_cursor(CursorMove::Back),
            KeyCode::Char('l') => self.textarea.move_cursor(CursorMove::Forward),
            _ => {}
        }
        Ok(())
    }

    fn handle_select_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.textarea.cancel_selection();
                self.mode = StudioMode::Normal;
                self.status_message = "Normal mode".into();
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.apply_transpose(1);
                self.mode = StudioMode::Normal;
                self.textarea.cancel_selection();
            }
            KeyCode::Char('>') => {
                self.apply_transpose(1);
                self.mode = StudioMode::Normal;
                self.textarea.cancel_selection();
            }
            KeyCode::Char('-') => {
                self.apply_transpose(-1);
                self.mode = StudioMode::Normal;
                self.textarea.cancel_selection();
            }
            KeyCode::Char('<') => {
                self.apply_transpose(-1);
                self.mode = StudioMode::Normal;
                self.textarea.cancel_selection();
            }
            KeyCode::Char(']') => {
                self.apply_transpose(12);
                self.mode = StudioMode::Normal;
                self.textarea.cancel_selection();
            }
            KeyCode::Char('[') => {
                self.apply_transpose(-12);
                self.mode = StudioMode::Normal;
                self.textarea.cancel_selection();
            }
            KeyCode::Up => self.textarea.move_cursor(CursorMove::Up),
            KeyCode::Down => self.textarea.move_cursor(CursorMove::Down),
            KeyCode::Left => self.textarea.move_cursor(CursorMove::Back),
            KeyCode::Right => self.textarea.move_cursor(CursorMove::Forward),
            KeyCode::Char('j') => self.textarea.move_cursor(CursorMove::Down),
            KeyCode::Char('k') => self.textarea.move_cursor(CursorMove::Up),
            KeyCode::Char('h') => self.textarea.move_cursor(CursorMove::Back),
            KeyCode::Char('l') => self.textarea.move_cursor(CursorMove::Forward),
            _ => {}
        }
        Ok(())
    }

    fn apply_transpose(&mut self, semitones: i32) {
        if let Err(e) = self.transpose_selection(semitones) {
            self.status_message = format!("Transpose failed: {}", e);
        }
    }

    fn handle_insert_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.code == KeyCode::Esc {
            self.mode = StudioMode::Normal;
            self.compile_and_update_current_source()?;
            return Ok(());
        }

        if self.textarea.input(key) {
            self.dirty = true;
        }
        Ok(())
    }

    fn handle_tick(&mut self, _event: Event) {}

    fn source(&self) -> String {
        let mut source = self.textarea.lines().join("\n");
        source.push('\n');
        source
    }

    fn replace_source(&mut self, source: String) {
        self.textarea = TextArea::from(source.lines());
        self.textarea
            .set_line_number_style(Style::default().fg(Color::DarkGray));
        self.textarea
            .set_cursor_line_style(Style::default().add_modifier(Modifier::REVERSED));
        self.textarea
            .set_selection_style(Style::default().bg(Color::Blue));
    }

    fn transpose_selection(&mut self, semitones: i32) -> Result<()> {
        let (start, end) = self.selected_line_range();
        let mut lines = self.textarea.lines().to_vec();
        let mut changed = 0usize;

        for row in start..=end {
            if let Some(line) = lines.get_mut(row) {
                let (new_line, line_changed) = transpose_line(line, semitones)?;
                if line_changed {
                    *line = new_line;
                    changed += 1;
                }
            }
        }

        if changed == 0 {
            self.status_message = "No transposable notes in selection".into();
            return Ok(());
        }

        let cursor = self.textarea.cursor();
        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.textarea
            .move_cursor(CursorMove::Jump(cursor.0 as u16, cursor.1 as u16));
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Transposed {} line{} by {:+} semitone{}",
            changed,
            if changed == 1 { "" } else { "s" },
            semitones,
            if semitones.abs() == 1 { "" } else { "s" }
        );
        Ok(())
    }

    fn push_source_undo(&mut self) {
        self.source_undo_stack.push(self.source());
        const MAX_SOURCE_UNDO: usize = 32;
        if self.source_undo_stack.len() > MAX_SOURCE_UNDO {
            self.source_undo_stack.remove(0);
        }
    }

    fn selected_line_range(&self) -> (usize, usize) {
        if let Some(((start_row, _), (end_row, end_col))) = self.textarea.selection_range() {
            let end_row = if end_col == 0 && end_row > start_row {
                end_row - 1
            } else {
                end_row
            };
            (start_row, end_row)
        } else {
            let row = self.textarea.cursor().0;
            (row, row)
        }
    }

    fn save(&mut self) -> Result<()> {
        fs::write(&self.path, self.source()).into_diagnostic()?;
        self.dirty = false;
        self.compile_and_update_current_source()?;
        self.status_message = format!("Saved {}", self.path.display());
        Ok(())
    }

    fn format_current_source(&mut self) -> Result<()> {
        match formatter::format_string(&self.source()) {
            Ok(formatted) => {
                let cursor = self.textarea.cursor();
                self.push_source_undo();
                self.replace_source(formatted);
                self.textarea
                    .move_cursor(CursorMove::Jump(cursor.0 as u16, cursor.1 as u16));
                self.dirty = true;
                self.compile_and_update_current_source()?;
                self.status_message = "Formatted".into();
            }
            Err(e) => {
                self.status_message = format!("Format failed: {}", e);
            }
        }
        Ok(())
    }

    fn compile_and_update_current_source(&mut self) -> Result<()> {
        match parser::parse_song(self.source()) {
            Ok(song) => match compiler::Compiler::new(&song) {
                Ok(compiler_inst) => match compiler_inst.compile(&song) {
                    Ok(events) => {
                        let events: Vec<crate::compiler::MidiEvent> = events.to_vec();
                        let bpm = song.metadata.bpm;
                        let note_count = events
                            .iter()
                            .filter(|e| matches!(e, crate::compiler::MidiEvent::Note { .. }))
                            .count();
                        let control_count = events
                            .iter()
                            .filter(|e| {
                                matches!(
                                    e,
                                    crate::compiler::MidiEvent::ControlChange { .. }
                                        | crate::compiler::MidiEvent::ProgramChange { .. }
                                )
                            })
                            .count();
                        self.bpm = bpm;
                        self.player.update(events, song.metadata);
                        self.status_message = format!(
                            "{} note events, {} control events, {} BPM",
                            note_count, control_count, bpm
                        );
                    }
                    Err(e) => {
                        self.status_message = format!("Compile error: {}", e);
                    }
                },
                Err(e) => {
                    self.status_message = format!("Compiler init error: {}", e);
                }
            },
            Err(e) => {
                self.status_message = format!("Parse error: {}", e);
            }
        }
        Ok(())
    }

    fn ui(&self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Min(8),
                    Constraint::Length(5),
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
        let dirty = if self.dirty { " modified" } else { "" };
        let title = format!("Score - {} [{}{}]", self.path.display(), mode, dirty);
        let block = Block::default().title(title).borders(Borders::ALL);
        let inner = block.inner(chunks[0]);
        f.render_widget(block, chunks[0]);
        f.render_widget(&self.textarea, inner);

        let beat_val = *self.current_beat.lock().unwrap();
        let status = Paragraph::new(format!(
            "Device: {}\nStatus: {}\nBPM: {}\nState: {}\nBeat: {:.2}\n{}",
            self.midi_device_name,
            self.status_message,
            self.bpm,
            if self.is_playing { "PLAYING" } else { "PAUSED" },
            beat_val,
            self.config_status
        ))
        .block(Block::default().title("Playback").borders(Borders::ALL))
        .style(
            if self.status_message.contains("error") || self.status_message.contains("failed") {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            },
        );
        f.render_widget(status, chunks[1]);

        let help = match self.mode {
            StudioMode::Normal => {
                "i ins | v select | +/- or <> transpose | [] octave | space play | f fmt | w save"
            }
            StudioMode::Insert => "Esc normal | type to edit | Ctrl+U undo | Ctrl+R redo",
            StudioMode::Select => "arrows/jk select | +/- or <> transpose | [] octave | Esc cancel",
        };
        let footer = Paragraph::new(help).block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    }
}

fn transpose_line(line: &str, semitones: i32) -> Result<(String, bool)> {
    let Some(pipe_idx) = line.find('|') else {
        return Ok((line.to_string(), false));
    };
    let (head, tail) = line.split_at(pipe_idx);
    if head.trim() == "seq" {
        let (new_tail, changed) = transpose_note_tokens(tail, semitones)?;
        if !changed {
            return Ok((line.to_string(), false));
        }
        return Ok((format!("{}{}", head, new_tail), true));
    }

    let (new_head, changed) = transpose_note_head(head, semitones)?;
    if !changed {
        return Ok((line.to_string(), false));
    }
    Ok((format!("{}{}", new_head, tail), true))
}

fn transpose_note_head(head: &str, semitones: i32) -> Result<(String, bool)> {
    transpose_note_tokens(head, semitones)
}

fn transpose_note_tokens(input: &str, semitones: i32) -> Result<(String, bool)> {
    let mut out = String::with_capacity(input.len());
    let mut token = String::new();
    let mut changed = false;

    for ch in input.chars() {
        if is_note_token_char(ch) {
            token.push(ch);
        } else {
            if !token.is_empty() {
                out.push_str(&transpose_note_token(&token, semitones, &mut changed)?);
                token.clear();
            }
            out.push(ch);
        }
    }

    if !token.is_empty() {
        out.push_str(&transpose_note_token(&token, semitones, &mut changed)?);
    }

    Ok((out, changed))
}

fn transpose_note_token(token: &str, semitones: i32, changed: &mut bool) -> Result<String> {
    let note = match token.parse::<Note>() {
        Ok(note) => note,
        Err(_) => return Ok(token.to_string()),
    };

    if matches!(note, Note::Drum(_)) {
        return Ok(token.to_string());
    }

    let midi = i32::from(note.to_midi_checked()?) + semitones;
    if !(0..=127).contains(&midi) {
        return Err(miette::miette!(
            "Transpose result out of MIDI range for {}: {}",
            token,
            midi
        ));
    }

    *changed = true;
    Ok(midi_to_loom_pitch(midi as u8))
}

fn midi_to_loom_pitch(midi: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = i32::from(midi / 12) - 2;
    let name = NAMES[usize::from(midi % 12)];
    format!("{}{}", name, octave)
}

fn is_note_token_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '#' || ch == '-'
}

#[cfg(test)]
mod tests {
    use super::transpose_line;

    #[test]
    fn transpose_pitch_list_before_bar() {
        let (line, changed) = transpose_line("F4,C5 | ^ . |", 2).unwrap();
        assert!(changed);
        assert_eq!(line, "G4,D5 | ^ . |");
    }

    #[test]
    fn transpose_keeps_drums() {
        let (line, changed) = transpose_line("kick | ^ . |", 2).unwrap();
        assert!(!changed);
        assert_eq!(line, "kick | ^ . |");
    }

    #[test]
    fn transpose_numeric_midi_to_pitch() {
        let (line, changed) = transpose_line("60 | ^ . |", 1).unwrap();
        assert!(changed);
        assert_eq!(line, "C#3 | ^ . |");
    }

    #[test]
    fn transpose_seq_body_notes() {
        let (line, changed) = transpose_line("seq | D4 . Eb4 A#3 |", 1).unwrap();
        assert!(changed);
        assert_eq!(line, "seq | D#4 . E4 B3 |");
    }
}

fn midi_device_name(port_index: usize) -> String {
    if let Ok(midi_out) = midir::MidiOutput::new("Loom Studio Info") {
        let ports = midi_out.ports();
        if let Some(port) = ports.get(port_index) {
            midi_out
                .port_name(port)
                .unwrap_or_else(|_| format!("Port {}", port_index))
        } else {
            format!("Port {} (Not Found)", port_index)
        }
    } else {
        format!("Port {}", port_index)
    }
}
