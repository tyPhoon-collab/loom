use crate::compiler;
use crate::dsl::note::Note;
use crate::dsl::{formatter, parser};
use crate::event::Event;
use crate::live_player::LivePlayer;
use crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
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

#[derive(Clone, Debug)]
enum CompileStatus {
    Ok {
        notes: usize,
        controls: usize,
        bpm: u32,
    },
    Error(String),
}

#[derive(Clone, Debug)]
enum StudioSelection {
    Note {
        row: usize,
        start_col: usize,
        end_col: usize,
        token: String,
    },
    NoteRange {
        anchor: NoteTokenSpan,
        focus: NoteTokenSpan,
    },
    LineRange {
        anchor_row: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NoteTokenSpan {
    row: usize,
    start_col: usize,
    end_col: usize,
    token: String,
}

pub struct StudioApp {
    should_quit: bool,
    path: PathBuf,
    mode: StudioMode,
    status_message: String,
    compile_status: CompileStatus,
    dirty: bool,
    is_playing: bool,
    bpm: u32,
    midi_device_name: String,
    config_status: String,
    current_beat: Arc<Mutex<f64>>,
    textarea: TextArea<'static>,
    selection: Option<StudioSelection>,
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
            compile_status: CompileStatus::Ok {
                notes: 0,
                controls: 0,
                bpm: 120,
            },
            dirty: false,
            is_playing: false,
            bpm: 120,
            midi_device_name,
            config_status,
            current_beat,
            textarea,
            selection: None,
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
                self.enter_note_select_mode();
            }
            KeyCode::Char('V') => {
                self.enter_line_select_mode();
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
                self.exit_select_mode();
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
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.expand_note_selection(-1);
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.expand_note_selection(1);
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.expand_note_selection_vertical(-1);
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.expand_note_selection_vertical(1);
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_selection_vertical(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection_vertical(1),
            KeyCode::Left | KeyCode::Char('h') => self.move_note_selection(-1),
            KeyCode::Right | KeyCode::Char('l') => self.move_note_selection(1),
            KeyCode::Char('H') => self.expand_note_selection(-1),
            KeyCode::Char('J') => self.expand_note_selection_vertical(1),
            KeyCode::Char('K') => self.expand_note_selection_vertical(-1),
            KeyCode::Char('L') => self.expand_note_selection(1),
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
            self.status_message = format!("Normal mode: {}", self.cursor_label());
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

    fn enter_line_select_mode(&mut self) {
        self.mode = StudioMode::Select;
        self.selection = Some(StudioSelection::LineRange {
            anchor_row: self.textarea.cursor().0,
        });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    fn enter_note_select_mode(&mut self) {
        let cursor = self.textarea.cursor();
        let Some(note) = self.note_at_or_after_cursor(cursor.0, cursor.1) else {
            self.status_message = "No note token on this line".into();
            return;
        };

        self.mode = StudioMode::Select;
        self.selection = Some(StudioSelection::Note {
            row: note.row,
            start_col: note.start_col,
            end_col: note.end_col,
            token: note.token,
        });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    fn exit_select_mode(&mut self) {
        self.selection = None;
        self.textarea.cancel_selection();
        self.mode = StudioMode::Normal;
        self.status_message = format!("Normal mode: {}", self.cursor_label());
    }

    fn move_note_selection(&mut self, direction: i32) {
        let Some(current) = self.focus_note() else {
            self.status_message = "No note selected. Press v on a note first.".into();
            return;
        };
        let notes = self.note_token_spans();
        let Some(index) = notes.iter().position(|note| note == &current) else {
            self.status_message = "Selected note no longer exists".into();
            return;
        };

        let next_index = if direction < 0 {
            index.checked_sub(1)
        } else {
            (index + 1 < notes.len()).then_some(index + 1)
        };

        let Some(next_index) = next_index else {
            self.status_message = "No more note tokens".into();
            return;
        };

        self.set_note_selection(notes[next_index].clone());
    }

    fn expand_note_selection(&mut self, direction: i32) {
        let Some(focus) = self.focus_note() else {
            self.status_message = "No note selected. Press v on a note first.".into();
            return;
        };
        let notes = self.note_token_spans();
        let Some(focus_index) = notes.iter().position(|note| note == &focus) else {
            self.status_message = "Selected note no longer exists".into();
            return;
        };

        let next_index = if direction < 0 {
            focus_index.checked_sub(1)
        } else {
            (focus_index + 1 < notes.len()).then_some(focus_index + 1)
        };

        let Some(next_index) = next_index else {
            self.status_message = "No more note tokens".into();
            return;
        };

        self.expand_note_selection_to(notes[next_index].clone());
    }

    fn expand_note_selection_vertical(&mut self, direction: i32) {
        let Some(focus) = self.focus_note() else {
            self.status_message = "No note selected. Press v on a note first.".into();
            return;
        };
        let next_row = if direction < 0 {
            focus.row.checked_sub(1)
        } else {
            (focus.row + 1 < self.textarea.lines().len()).then_some(focus.row + 1)
        };
        let Some(next_row) = next_row else {
            self.status_message = "No more lines".into();
            return;
        };
        let Some(note) = self.nearest_note_on_line(next_row, focus.start_col) else {
            self.status_message = "No note token on target line".into();
            return;
        };
        self.expand_note_selection_to(note);
    }

    fn move_selection_vertical(&mut self, direction: i32) {
        match self.selection {
            Some(StudioSelection::LineRange { .. }) => {
                let cursor_move = if direction < 0 {
                    CursorMove::Up
                } else {
                    CursorMove::Down
                };
                self.textarea.move_cursor(cursor_move);
                self.sync_selection_visual();
                self.status_message = format!("Select mode: {}", self.selection_label());
            }
            Some(StudioSelection::Note { row, start_col, .. })
            | Some(StudioSelection::NoteRange {
                focus: NoteTokenSpan { row, start_col, .. },
                ..
            }) => {
                let next_row = if direction < 0 {
                    row.checked_sub(1)
                } else {
                    (row + 1 < self.textarea.lines().len()).then_some(row + 1)
                };
                let Some(next_row) = next_row else {
                    self.status_message = "No more lines".into();
                    return;
                };
                let Some(note) = self.nearest_note_on_line(next_row, start_col) else {
                    self.status_message = "No note token on target line".into();
                    return;
                };
                self.set_note_selection(note);
            }
            None => {
                self.status_message = "No selection".into();
            }
        }
    }

    fn set_note_selection(&mut self, note: NoteTokenSpan) {
        self.selection = Some(StudioSelection::Note {
            row: note.row,
            start_col: note.start_col,
            end_col: note.end_col,
            token: note.token,
        });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    fn expand_note_selection_to(&mut self, focus: NoteTokenSpan) {
        let anchor = match &self.selection {
            Some(StudioSelection::Note {
                row,
                start_col,
                end_col,
                token,
            }) => NoteTokenSpan {
                row: *row,
                start_col: *start_col,
                end_col: *end_col,
                token: token.clone(),
            },
            Some(StudioSelection::NoteRange { anchor, .. }) => anchor.clone(),
            _ => {
                self.status_message = "Current selection is not a note selection".into();
                return;
            }
        };

        self.selection = Some(StudioSelection::NoteRange { anchor, focus });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    fn sync_selection_visual(&mut self) {
        let Some(selection) = self.selection.clone() else {
            return;
        };

        self.textarea.cancel_selection();
        match selection {
            StudioSelection::Note {
                row,
                start_col,
                end_col,
                ..
            } => {
                self.textarea
                    .move_cursor(CursorMove::Jump(row as u16, start_col as u16));
                self.textarea.start_selection();
                self.textarea
                    .move_cursor(CursorMove::Jump(row as u16, end_col as u16));
            }
            StudioSelection::NoteRange { anchor, focus } => {
                let (start, end) = ordered_note_span_bounds(&anchor, &focus);
                self.textarea
                    .move_cursor(CursorMove::Jump(start.row as u16, start.start_col as u16));
                self.textarea.start_selection();
                self.textarea
                    .move_cursor(CursorMove::Jump(end.row as u16, end.end_col as u16));
            }
            StudioSelection::LineRange { anchor_row } => {
                let current_row = self.textarea.cursor().0;
                let anchor_col = self.line_len(anchor_row);
                let current_col = self.line_len(current_row);

                if current_row >= anchor_row {
                    self.textarea
                        .move_cursor(CursorMove::Jump(anchor_row as u16, 0));
                    self.textarea.start_selection();
                    self.textarea
                        .move_cursor(CursorMove::Jump(current_row as u16, current_col as u16));
                } else {
                    self.textarea
                        .move_cursor(CursorMove::Jump(anchor_row as u16, anchor_col as u16));
                    self.textarea.start_selection();
                    self.textarea
                        .move_cursor(CursorMove::Jump(current_row as u16, 0));
                }
            }
        }
    }

    fn line_len(&self, row: usize) -> usize {
        self.textarea
            .lines()
            .get(row)
            .map(|line| line.chars().count())
            .unwrap_or(0)
    }

    fn transpose_selection(&mut self, semitones: i32) -> Result<()> {
        if matches!(
            self.selection,
            Some(StudioSelection::Note { .. } | StudioSelection::NoteRange { .. })
        ) {
            return self.transpose_selected_notes(semitones);
        }

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
        if self.selection.is_some() {
            self.sync_selection_visual();
        }
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

    fn transpose_selected_notes(&mut self, semitones: i32) -> Result<()> {
        let selected_indices = self.selected_note_indices();
        let selected_notes = self.selected_note_spans();
        if selected_notes.is_empty() {
            self.status_message = "No note selected".into();
            return Ok(());
        }

        let mut replacements = Vec::new();
        for note in selected_notes {
            let mut changed = false;
            let new_token = transpose_note_token(&note.token, semitones, &mut changed)?;
            if changed {
                replacements.push((note, new_token));
            }
        }

        if replacements.is_empty() {
            self.status_message = "No transposable note selected".into();
            return Ok(());
        }

        let mut lines = self.textarea.lines().to_vec();
        replacements.sort_by(|(left, _), (right, _)| {
            right
                .row
                .cmp(&left.row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });
        for (note, new_token) in &replacements {
            let Some(line) = lines.get_mut(note.row) else {
                self.status_message = "Selected note no longer exists".into();
                return Ok(());
            };
            replace_char_range(line, note.start_col, note.end_col, new_token);
        }

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_note_selection_from_indices(&selected_indices);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Transposed {} note{} by {:+}",
            replacements.len(),
            if replacements.len() == 1 { "" } else { "s" },
            semitones
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
        match &self.selection {
            Some(StudioSelection::LineRange { anchor_row }) => {
                let row = self.textarea.cursor().0;
                ((*anchor_row).min(row), (*anchor_row).max(row))
            }
            Some(StudioSelection::Note { row, .. }) => (*row, *row),
            Some(StudioSelection::NoteRange { anchor, focus }) => {
                (anchor.row.min(focus.row), anchor.row.max(focus.row))
            }
            None => {
                let row = self.textarea.cursor().0;
                (row, row)
            }
        }
    }

    fn cursor_label(&self) -> String {
        let cursor = self.textarea.cursor();
        format!("line {}, col {}", cursor.0 + 1, cursor.1 + 1)
    }

    fn selection_label(&self) -> String {
        match &self.selection {
            Some(StudioSelection::Note {
                row,
                start_col,
                token,
                ..
            }) => format!("note {} at line {}, col {}", token, row + 1, start_col + 1),
            Some(StudioSelection::NoteRange { .. }) => {
                let selected = self.selected_note_spans();
                match (selected.first(), selected.last()) {
                    (Some(first), Some(last)) if selected.len() == 1 => {
                        format!(
                            "note {} at line {}, col {}",
                            first.token,
                            first.row + 1,
                            first.start_col + 1
                        )
                    }
                    (Some(first), Some(last)) => format!(
                        "{} notes from {} to {}",
                        selected.len(),
                        first.token,
                        last.token
                    ),
                    _ => "no notes".to_string(),
                }
            }
            Some(StudioSelection::LineRange { .. }) | None => {
                let (start, end) = self.selected_line_range();
                if start == end {
                    format!("line {}", start + 1)
                } else {
                    format!("lines {}..{}", start + 1, end + 1)
                }
            }
        }
    }

    fn focus_note(&self) -> Option<NoteTokenSpan> {
        match &self.selection {
            Some(StudioSelection::Note {
                row,
                start_col,
                end_col,
                token,
            }) => Some(NoteTokenSpan {
                row: *row,
                start_col: *start_col,
                end_col: *end_col,
                token: token.clone(),
            }),
            Some(StudioSelection::NoteRange { focus, .. }) => Some(focus.clone()),
            _ => None,
        }
    }

    fn selected_note_indices(&self) -> Vec<usize> {
        let notes = self.note_token_spans();
        match &self.selection {
            Some(StudioSelection::Note {
                row,
                start_col,
                end_col,
                token,
            }) => {
                let selected = NoteTokenSpan {
                    row: *row,
                    start_col: *start_col,
                    end_col: *end_col,
                    token: token.clone(),
                };
                notes
                    .iter()
                    .position(|note| note == &selected)
                    .map(|index| vec![index])
                    .unwrap_or_default()
            }
            Some(StudioSelection::NoteRange { anchor, focus }) => {
                let Some(anchor_index) = notes.iter().position(|note| note == anchor) else {
                    return Vec::new();
                };
                let Some(focus_index) = notes.iter().position(|note| note == focus) else {
                    return Vec::new();
                };
                let start = anchor_index.min(focus_index);
                let end = anchor_index.max(focus_index);
                (start..=end).collect()
            }
            _ => Vec::new(),
        }
    }

    fn selected_note_spans(&self) -> Vec<NoteTokenSpan> {
        let notes = self.note_token_spans();
        self.selected_note_indices()
            .into_iter()
            .filter_map(|index| notes.get(index).cloned())
            .collect()
    }

    fn restore_note_selection_from_indices(&mut self, selected_indices: &[usize]) {
        let notes = self.note_token_spans();
        match selected_indices {
            [] => {
                self.selection = None;
            }
            [index] => {
                if let Some(note) = notes.get(*index) {
                    self.selection = Some(StudioSelection::Note {
                        row: note.row,
                        start_col: note.start_col,
                        end_col: note.end_col,
                        token: note.token.clone(),
                    });
                }
            }
            indices => {
                let Some(first) = indices.first().and_then(|index| notes.get(*index)) else {
                    self.selection = None;
                    return;
                };
                let Some(last) = indices.last().and_then(|index| notes.get(*index)) else {
                    self.selection = None;
                    return;
                };
                self.selection = Some(StudioSelection::NoteRange {
                    anchor: first.clone(),
                    focus: last.clone(),
                });
            }
        }
    }

    fn note_at_or_after_cursor(&self, row: usize, col: usize) -> Option<NoteTokenSpan> {
        note_at_or_near_col(self.note_spans_on_line(row), col)
    }

    fn nearest_note_on_line(&self, row: usize, col: usize) -> Option<NoteTokenSpan> {
        self.note_spans_on_line(row)
            .into_iter()
            .min_by_key(|note| note.start_col.abs_diff(col))
    }

    fn note_token_spans(&self) -> Vec<NoteTokenSpan> {
        self.textarea
            .lines()
            .iter()
            .enumerate()
            .flat_map(|(row, _)| self.note_spans_on_line(row))
            .collect()
    }

    fn note_spans_on_line(&self, row: usize) -> Vec<NoteTokenSpan> {
        self.textarea
            .lines()
            .get(row)
            .map(|line| note_spans_in_line(row, line))
            .unwrap_or_default()
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
                        self.compile_status = CompileStatus::Ok {
                            notes: note_count,
                            controls: control_count,
                            bpm,
                        };
                    }
                    Err(e) => {
                        self.compile_status = CompileStatus::Error(format!("Compile error: {}", e));
                    }
                },
                Err(e) => {
                    self.compile_status =
                        CompileStatus::Error(format!("Compiler init error: {}", e));
                }
            },
            Err(e) => {
                self.compile_status = CompileStatus::Error(format!("Parse error: {}", e));
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
            "Device: {}\nPlayback: {}  Beat: {:.2}  BPM: {}\n{}\n{}\nMessage: {}\n{}",
            self.midi_device_name,
            playback_state,
            beat_val,
            self.bpm,
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

        let help = match self.mode {
            StudioMode::Normal => {
                "i ins | v note select | V line select | +/- transpose | [] octave | space play | w save"
            }
            StudioMode::Insert => "Esc normal | type to edit | Ctrl+U undo | Ctrl+R redo",
            StudioMode::Select => {
                "h/l move note | H/L extend notes | j/k vertical | +/- transpose | Esc cancel"
            }
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

fn note_spans_in_line(row: usize, line: &str) -> Vec<NoteTokenSpan> {
    let Some(pipe_col) = line.chars().position(|ch| ch == '|') else {
        return Vec::new();
    };

    let head: String = line.chars().take(pipe_col).collect();
    let scan_start_col = if head.trim() == "seq" {
        pipe_col + 1
    } else {
        0
    };
    let scan_end_col = if head.trim() == "seq" {
        line.chars().count()
    } else {
        pipe_col
    };

    let mut spans = Vec::new();
    let mut token = String::new();
    let mut token_start = 0usize;

    for (col, ch) in line.chars().enumerate() {
        if col < scan_start_col || col >= scan_end_col {
            if !token.is_empty() {
                push_note_span(&mut spans, row, token_start, col, &token);
                token.clear();
            }
            continue;
        }

        if is_note_token_char(ch) {
            if token.is_empty() {
                token_start = col;
            }
            token.push(ch);
        } else if !token.is_empty() {
            push_note_span(&mut spans, row, token_start, col, &token);
            token.clear();
        }
    }

    if !token.is_empty() {
        push_note_span(&mut spans, row, token_start, line.chars().count(), &token);
    }

    spans
}

fn note_at_or_near_col(notes: Vec<NoteTokenSpan>, col: usize) -> Option<NoteTokenSpan> {
    notes
        .iter()
        .find(|note| col >= note.start_col && col < note.end_col)
        .cloned()
        .or_else(|| notes.iter().find(|note| note.start_col >= col).cloned())
        .or_else(|| notes.into_iter().next_back())
}

fn ordered_note_span_bounds<'a>(
    left: &'a NoteTokenSpan,
    right: &'a NoteTokenSpan,
) -> (&'a NoteTokenSpan, &'a NoteTokenSpan) {
    if (left.row, left.start_col) <= (right.row, right.start_col) {
        (left, right)
    } else {
        (right, left)
    }
}

fn push_note_span(
    spans: &mut Vec<NoteTokenSpan>,
    row: usize,
    start_col: usize,
    end_col: usize,
    token: &str,
) {
    let Ok(note) = token.parse::<Note>() else {
        return;
    };
    if matches!(note, Note::Drum(_)) {
        return;
    }

    spans.push(NoteTokenSpan {
        row,
        start_col,
        end_col,
        token: token.to_string(),
    });
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

fn replace_char_range(line: &mut String, start_col: usize, end_col: usize, replacement: &str) {
    let start = char_to_byte_index(line, start_col);
    let end = char_to_byte_index(line, end_col);
    line.replace_range(start..end, replacement);
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
    use super::{note_at_or_near_col, note_spans_in_line, replace_char_range, transpose_line};

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

    #[test]
    fn note_spans_seq_body_only() {
        let spans = note_spans_in_line(0, "seq | D4 . Eb4 A#3 |");
        let tokens: Vec<_> = spans.iter().map(|span| span.token.as_str()).collect();
        assert_eq!(tokens, vec!["D4", "Eb4", "A#3"]);
    }

    #[test]
    fn note_spans_note_head_only() {
        let spans = note_spans_in_line(0, "F4,C5 | ^ . |");
        let tokens: Vec<_> = spans.iter().map(|span| span.token.as_str()).collect();
        assert_eq!(tokens, vec!["F4", "C5"]);
    }

    #[test]
    fn replace_selected_note_token() {
        let mut line = "seq | D4 . Eb4 |".to_string();
        let span = note_spans_in_line(0, &line)
            .into_iter()
            .find(|span| span.token == "Eb4")
            .unwrap();
        replace_char_range(&mut line, span.start_col, span.end_col, "E4");
        assert_eq!(line, "seq | D4 . E4 |");
    }

    #[test]
    fn note_select_uses_note_under_cursor() {
        let notes = note_spans_in_line(0, "seq | D4 . Eb4 |");
        let selected = note_at_or_near_col(notes, 6).unwrap();
        assert_eq!(selected.token, "D4");
    }

    #[test]
    fn note_select_falls_forward() {
        let notes = note_spans_in_line(0, "seq | D4 . Eb4 |");
        let selected = note_at_or_near_col(notes, 9).unwrap();
        assert_eq!(selected.token, "Eb4");
    }

    #[test]
    fn note_select_falls_back_to_last_note() {
        let notes = note_spans_in_line(0, "seq | D4 . Eb4 |");
        let selected = note_at_or_near_col(notes, 99).unwrap();
        assert_eq!(selected.token, "Eb4");
    }

    #[test]
    fn ordered_note_span_bounds_sorts_by_position() {
        let notes = note_spans_in_line(0, "seq | D4 . Eb4 |");
        let (start, end) = super::ordered_note_span_bounds(&notes[1], &notes[0]);
        assert_eq!(start.token, "D4");
        assert_eq!(end.token, "Eb4");
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
