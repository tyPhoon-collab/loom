use crate::compiler;
use crate::dsl::note::Note;
use crate::dsl::{formatter, parser};
use crate::event::Event;
use crate::live_player::LivePlayer;
use crate::sequencer::PlaybackState;
use crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use miette::{IntoDiagnostic, Result};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_textarea::CursorMove;
use ratatui_textarea::TextArea;
use selection::{
    bar_at_or_near_col, bar_spans_in_line, char_range, delete_note_token, insert_at_col,
    is_note_token_char, is_seq_line, note_at_or_near_col, note_spans_in_line,
    ordered_bar_span_bounds, ordered_note_span_bounds, replace_char_range, BarSpan, NoteTokenSpan,
    SelectableTokenKind, StudioSelection,
};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod selection;

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

pub struct StudioApp {
    should_quit: bool,
    path: PathBuf,
    mode: StudioMode,
    pending_add: bool,
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
            pending_add: false,
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
        if self.pending_add {
            self.pending_add = false;
            return self.handle_add_key(key);
        }

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
            KeyCode::Char('a') => {
                self.pending_add = true;
                self.status_message =
                    "Add: s seq | l note-head | t track | b bar | n note | . rest | - sustain"
                        .into();
            }
            KeyCode::Char('v') => {
                self.enter_note_select_mode();
            }
            KeyCode::Char('V') => {
                self.enter_line_select_mode();
            }
            KeyCode::Char('b') => {
                self.enter_bar_select_mode();
            }
            KeyCode::Char('B') => {
                self.enter_line_bar_select_mode();
            }
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.clear_loop_settings()?;
            }
            KeyCode::Char('L') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.clear_loop_settings()?;
                } else {
                    self.toggle_loop()?;
                }
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

    fn handle_add_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.status_message = "Add cancelled".into();
            }
            KeyCode::Char('s') => {
                self.add_seq_line()?;
            }
            KeyCode::Char('l') => {
                self.add_note_head_line()?;
            }
            KeyCode::Char('t') => {
                self.add_track()?;
            }
            KeyCode::Char('b') => {
                self.add_bar()?;
            }
            KeyCode::Char('n') => {
                let token = self.note_token_for_add();
                self.place_token_at_current_slot(&token)?;
            }
            KeyCode::Char('.') => {
                self.place_token_at_current_slot(".")?;
            }
            KeyCode::Char('-') => {
                self.place_token_at_current_slot("-")?;
            }
            _ => {
                self.status_message = "Unknown add command. Use s, l, t, b, n, ., or -".into();
            }
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
            KeyCode::Char('x') => {
                self.delete_selection()?;
            }
            KeyCode::Char('r') => {
                self.replace_selected_tokens(".")?;
            }
            KeyCode::Char('s') => {
                self.replace_selected_tokens("-")?;
            }
            KeyCode::Char('d') => {
                self.duplicate_selection()?;
            }
            KeyCode::Enter => {
                self.apply_selected_loop_range()?;
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.expand_selection_horizontal(-1);
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.expand_selection_horizontal(1);
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.expand_selection_vertical(-1);
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.expand_selection_vertical(1);
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_selection_vertical(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection_vertical(1),
            KeyCode::Left | KeyCode::Char('h') => self.move_selection_horizontal(-1),
            KeyCode::Right | KeyCode::Char('l') => self.move_selection_horizontal(1),
            KeyCode::Char('H') => self.expand_selection_horizontal(-1),
            KeyCode::Char('J') => self.expand_selection_vertical(1),
            KeyCode::Char('K') => self.expand_selection_vertical(-1),
            KeyCode::Char('L') => self.expand_selection_horizontal(1),
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

    fn handle_tick(&mut self, _event: Event) {
        self.sync_playback_state();
    }

    fn sync_playback_state(&mut self) {
        self.is_playing = self.player.playback_state() == PlaybackState::Playing;
    }

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
            self.status_message = "No editable token on this line".into();
            return;
        };

        self.mode = StudioMode::Select;
        self.selection = Some(StudioSelection::Note {
            row: note.row,
            start_col: note.start_col,
            end_col: note.end_col,
            token: note.token,
            kind: note.kind,
        });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    fn enter_bar_select_mode(&mut self) {
        let cursor = self.textarea.cursor();
        let Some(bar) = self.bar_at_or_after_cursor(cursor.0, cursor.1) else {
            self.status_message = "No bar on this line".into();
            return;
        };

        self.mode = StudioMode::Select;
        self.selection = Some(StudioSelection::Bar { span: bar });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    fn enter_line_bar_select_mode(&mut self) {
        let row = self.textarea.cursor().0;
        let bars = self.bar_spans_on_line(row);
        match (bars.first(), bars.last()) {
            (Some(first), Some(last)) if first == last => {
                self.mode = StudioMode::Select;
                self.selection = Some(StudioSelection::Bar {
                    span: first.clone(),
                });
            }
            (Some(first), Some(last)) => {
                self.mode = StudioMode::Select;
                self.selection = Some(StudioSelection::BarRange {
                    anchor: first.clone(),
                    focus: last.clone(),
                });
            }
            _ => {
                self.status_message = "No bars on this line".into();
                return;
            }
        }
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    fn exit_select_mode(&mut self) {
        self.selection = None;
        self.textarea.cancel_selection();
        self.mode = StudioMode::Normal;
        self.status_message = format!("Normal mode: {}", self.cursor_label());
    }

    fn move_selection_horizontal(&mut self, direction: i32) {
        match self.selection {
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. }) => {
                self.move_bar_selection(direction);
            }
            _ => self.move_note_selection(direction),
        }
    }

    fn expand_selection_horizontal(&mut self, direction: i32) {
        match self.selection {
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. }) => {
                self.expand_bar_selection(direction);
            }
            _ => self.expand_note_selection(direction),
        }
    }

    fn expand_selection_vertical(&mut self, direction: i32) {
        match self.selection {
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. }) => {
                self.expand_bar_selection_vertical(direction);
            }
            _ => self.expand_note_selection_vertical(direction),
        }
    }

    fn move_note_selection(&mut self, direction: i32) {
        let Some(current) = self.focus_note() else {
            self.status_message = "No editable token selected. Press v first.".into();
            return;
        };
        let notes = self.note_token_spans();
        let Some(index) = notes.iter().position(|note| note == &current) else {
            self.status_message = "Selected token no longer exists".into();
            return;
        };

        let next_index = if direction < 0 {
            index.checked_sub(1)
        } else {
            (index + 1 < notes.len()).then_some(index + 1)
        };

        let Some(next_index) = next_index else {
            self.status_message = "No more editable tokens".into();
            return;
        };

        self.set_note_selection(notes[next_index].clone());
    }

    fn expand_note_selection(&mut self, direction: i32) {
        let Some(focus) = self.focus_note() else {
            self.status_message = "No editable token selected. Press v first.".into();
            return;
        };
        let notes = self.note_token_spans();
        let Some(focus_index) = notes.iter().position(|note| note == &focus) else {
            self.status_message = "Selected token no longer exists".into();
            return;
        };

        let next_index = if direction < 0 {
            focus_index.checked_sub(1)
        } else {
            (focus_index + 1 < notes.len()).then_some(focus_index + 1)
        };

        let Some(next_index) = next_index else {
            self.status_message = "No more editable tokens".into();
            return;
        };

        self.expand_note_selection_to(notes[next_index].clone());
    }

    fn expand_note_selection_vertical(&mut self, direction: i32) {
        let Some(focus) = self.focus_note() else {
            self.status_message = "No editable token selected. Press v first.".into();
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
            self.status_message = "No editable token on target line".into();
            return;
        };
        self.expand_note_selection_to(note);
    }

    fn move_bar_selection(&mut self, direction: i32) {
        let Some(current) = self.focus_bar() else {
            self.status_message = "No bar selected. Press b first.".into();
            return;
        };
        let bars = self.bar_spans();
        let Some(index) = bars.iter().position(|bar| bar == &current) else {
            self.status_message = "Selected bar no longer exists".into();
            return;
        };

        let next_index = if direction < 0 {
            index.checked_sub(1)
        } else {
            (index + 1 < bars.len()).then_some(index + 1)
        };

        let Some(next_index) = next_index else {
            self.status_message = "No more bars".into();
            return;
        };

        self.set_bar_selection(bars[next_index].clone());
    }

    fn expand_bar_selection(&mut self, direction: i32) {
        let Some(focus) = self.focus_bar() else {
            self.status_message = "No bar selected. Press b first.".into();
            return;
        };
        let bars = self.bar_spans();
        let Some(focus_index) = bars.iter().position(|bar| bar == &focus) else {
            self.status_message = "Selected bar no longer exists".into();
            return;
        };

        let next_index = if direction < 0 {
            focus_index.checked_sub(1)
        } else {
            (focus_index + 1 < bars.len()).then_some(focus_index + 1)
        };

        let Some(next_index) = next_index else {
            self.status_message = "No more bars".into();
            return;
        };

        self.expand_bar_selection_to(bars[next_index].clone());
    }

    fn expand_bar_selection_vertical(&mut self, direction: i32) {
        let Some(focus) = self.focus_bar() else {
            self.status_message = "No bar selected. Press b first.".into();
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
        let Some(bar) = self.nearest_bar_on_line(next_row, focus.start_col) else {
            self.status_message = "No bar on target line".into();
            return;
        };
        self.expand_bar_selection_to(bar);
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
            Some(StudioSelection::Bar {
                span: BarSpan { row, start_col, .. },
            })
            | Some(StudioSelection::BarRange {
                focus: BarSpan { row, start_col, .. },
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
                let Some(bar) = self.nearest_bar_on_line(next_row, start_col) else {
                    self.status_message = "No bar on target line".into();
                    return;
                };
                self.set_bar_selection(bar);
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
                    self.status_message = "No editable token on target line".into();
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
            kind: note.kind,
        });
        self.sync_selection_visual();
        self.status_message = format!("Select mode: {}", self.selection_label());
    }

    fn set_bar_selection(&mut self, bar: BarSpan) {
        self.selection = Some(StudioSelection::Bar { span: bar });
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
                kind,
            }) => NoteTokenSpan {
                row: *row,
                start_col: *start_col,
                end_col: *end_col,
                token: token.clone(),
                kind: *kind,
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

    fn expand_bar_selection_to(&mut self, focus: BarSpan) {
        let anchor = match &self.selection {
            Some(StudioSelection::Bar { span }) => span.clone(),
            Some(StudioSelection::BarRange { anchor, .. }) => anchor.clone(),
            _ => {
                self.status_message = "Current selection is not a bar selection".into();
                return;
            }
        };

        self.selection = Some(StudioSelection::BarRange { anchor, focus });
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
            StudioSelection::Bar { span } => {
                self.textarea
                    .move_cursor(CursorMove::Jump(span.row as u16, span.start_col as u16));
                self.textarea.start_selection();
                self.textarea
                    .move_cursor(CursorMove::Jump(span.row as u16, span.end_col as u16));
            }
            StudioSelection::BarRange { anchor, focus } => {
                let (start, end) = ordered_bar_span_bounds(&anchor, &focus);
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
        if matches!(
            self.selection,
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. })
        ) {
            return self.transpose_selected_bars(semitones);
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

        let cursor = self.textarea.cursor();
        let audition = self.audition_candidate_from_lines(&lines, start, end, (cursor.0, cursor.1));

        if changed == 0 {
            self.status_message = "No transposable notes in selection".into();
            return Ok(());
        }

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
        self.audition_candidate(audition);
        Ok(())
    }

    fn transpose_selected_bars(&mut self, semitones: i32) -> Result<()> {
        let selected_bars = self.selected_bar_spans();
        if selected_bars.is_empty() {
            self.status_message = "No bar selected".into();
            return Ok(());
        }

        let mut lines = self.textarea.lines().to_vec();
        let mut replacements = Vec::new();
        for bar in &selected_bars {
            let Some(line) = lines.get(bar.row) else {
                self.status_message = "Selected bar no longer exists".into();
                return Ok(());
            };
            let bar_text = char_range(line, bar.start_col, bar.end_col);
            let (new_bar_text, changed) = transpose_bar_text(&bar_text, semitones)?;
            if changed {
                replacements.push((bar.clone(), new_bar_text));
            }
        }

        if replacements.is_empty() {
            self.status_message = "No transposable notes in selected bars".into();
            return Ok(());
        }

        replacements.sort_by(|(left, _), (right, _)| {
            right
                .row
                .cmp(&left.row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });
        for (bar, new_bar_text) in &replacements {
            let Some(line) = lines.get_mut(bar.row) else {
                self.status_message = "Selected bar no longer exists".into();
                return Ok(());
            };
            replace_char_range(line, bar.start_col, bar.end_col, new_bar_text);
        }

        let selected_rows_indices: Vec<(usize, usize)> = selected_bars
            .iter()
            .map(|bar| (bar.row, bar.index))
            .collect();
        let audition = replacements.iter().find_map(|(old_bar, _)| {
            let line = lines.get(old_bar.row)?;
            let new_bar = bar_spans_in_line(old_bar.row, line)
                .into_iter()
                .find(|bar| bar.index == old_bar.index)?;
            note_spans_in_line(old_bar.row, line)
                .into_iter()
                .find(|note| {
                    note.kind == SelectableTokenKind::Note
                        && note.start_col >= new_bar.start_col
                        && note.end_col <= new_bar.end_col
                })
                .map(|note| (note.row, note.token))
        });

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_bar_selection_from_positions(&selected_rows_indices);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Transposed {} bar{} by {:+}",
            selected_bars.len(),
            if selected_bars.len() == 1 { "" } else { "s" },
            semitones
        );
        self.audition_candidate(audition);
        Ok(())
    }

    fn transpose_selected_notes(&mut self, semitones: i32) -> Result<()> {
        let selected_indices = self.selected_note_indices();
        let selected_notes = self.selected_note_spans();
        if selected_notes.is_empty() {
            self.status_message = "No editable token selected".into();
            return Ok(());
        }

        let mut replacements = Vec::new();
        for note in selected_notes {
            if note.kind != SelectableTokenKind::Note {
                continue;
            }
            let mut changed = false;
            let new_token = transpose_note_token(&note.token, semitones, &mut changed)?;
            if changed {
                replacements.push((note, new_token));
            }
        }

        let audition = replacements
            .first()
            .map(|(note, token)| (note.row, token.clone()));

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
                self.status_message = "Selected token no longer exists".into();
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
        self.audition_candidate(audition);
        Ok(())
    }

    fn replace_selected_tokens(&mut self, replacement: &str) -> Result<()> {
        let selected_indices = self.selected_note_indices();
        let mut selected_notes = self.selected_note_spans();
        if selected_notes.is_empty() {
            self.status_message = "Replacement applies to editable token selection only".into();
            return Ok(());
        }

        let replacement_name = match replacement {
            "." => "Rested",
            "-" => "Sustained",
            _ => "Replaced",
        };
        selected_notes.sort_by(|left, right| {
            right
                .row
                .cmp(&left.row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });

        let mut lines = self.textarea.lines().to_vec();
        for note in &selected_notes {
            let Some(line) = lines.get_mut(note.row) else {
                self.status_message = "Selected token no longer exists".into();
                return Ok(());
            };
            replace_char_range(line, note.start_col, note.end_col, replacement);
        }

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_note_selection_from_indices(&selected_indices);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "{} {} token{}",
            replacement_name,
            selected_notes.len(),
            if selected_notes.len() == 1 { "" } else { "s" }
        );
        Ok(())
    }

    fn delete_selected_notes(&mut self) -> Result<()> {
        let mut selected_notes = self.selected_note_spans();
        if selected_notes.is_empty() {
            self.status_message = "x applies to editable token selection only".into();
            return Ok(());
        }

        let first = selected_notes.first().cloned();
        selected_notes.sort_by(|left, right| {
            right
                .row
                .cmp(&left.row)
                .then_with(|| right.start_col.cmp(&left.start_col))
        });

        let mut lines = self.textarea.lines().to_vec();
        for note in &selected_notes {
            let Some(line) = lines.get_mut(note.row) else {
                self.status_message = "Selected token no longer exists".into();
                return Ok(());
            };
            delete_note_token(line, note);
        }

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        if let Some(note) = first {
            self.textarea
                .move_cursor(CursorMove::Jump(note.row as u16, note.start_col as u16));
        }
        self.selection = None;
        self.textarea.cancel_selection();
        self.mode = StudioMode::Normal;
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Deleted {} token{}",
            selected_notes.len(),
            if selected_notes.len() == 1 { "" } else { "s" }
        );
        Ok(())
    }

    fn delete_selection(&mut self) -> Result<()> {
        if matches!(
            self.selection,
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. })
        ) {
            self.delete_selected_bars()
        } else {
            self.delete_selected_notes()
        }
    }

    fn delete_selected_bars(&mut self) -> Result<()> {
        let selected_bars = self.selected_bar_spans();
        if selected_bars.is_empty() {
            self.status_message = "x applies to bar selection only".into();
            return Ok(());
        }

        let row = selected_bars[0].row;
        if selected_bars.iter().any(|bar| bar.row != row) {
            self.status_message = "Bar delete currently supports one line at a time".into();
            return Ok(());
        }
        if selected_bars.len() == self.bar_spans_on_line(row).len() {
            self.status_message = "Cannot delete all bars on a line".into();
            return Ok(());
        }

        let first = selected_bars.first().cloned().unwrap();
        let last = selected_bars.last().cloned().unwrap();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(row) else {
            self.status_message = "Selected bar no longer exists".into();
            return Ok(());
        };
        replace_char_range(line, first.start_col + 1, last.end_col, "");

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.textarea
            .move_cursor(CursorMove::Jump(row as u16, first.start_col as u16));
        self.selection = None;
        self.textarea.cancel_selection();
        self.mode = StudioMode::Normal;
        self.dirty = true;
        self.compile_and_update_current_source()?;
        let mut status_message = format!(
            "Deleted {} bar{}",
            selected_bars.len(),
            if selected_bars.len() == 1 { "" } else { "s" }
        );
        if self.current_loop_range().is_some() {
            status_message.push_str(" | loop_range unchanged");
        }
        self.status_message = status_message;
        Ok(())
    }

    fn duplicate_selection(&mut self) -> Result<()> {
        if matches!(
            self.selection,
            Some(StudioSelection::Bar { .. } | StudioSelection::BarRange { .. })
        ) {
            self.duplicate_selected_bars()
        } else {
            self.duplicate_selected_notes()
        }
    }

    fn duplicate_selected_notes(&mut self) -> Result<()> {
        let selected_indices = self.selected_note_indices();
        let selected_notes = self.selected_note_spans();
        if selected_notes.is_empty() {
            self.status_message = "d applies to editable token selection only".into();
            return Ok(());
        }

        let row = selected_notes[0].row;
        if selected_notes.iter().any(|note| note.row != row) {
            self.status_message = "Duplicate currently supports one seq line at a time".into();
            return Ok(());
        }

        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(row) else {
            self.status_message = "Selected token no longer exists".into();
            return Ok(());
        };
        if !is_seq_line(line) {
            self.status_message = "Duplicate currently supports seq body tokens only".into();
            return Ok(());
        }

        let Some(last_note) = selected_notes.last() else {
            return Ok(());
        };
        let insertion = format!(
            " {}",
            selected_notes
                .iter()
                .map(|note| note.token.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        );
        insert_at_col(line, last_note.end_col, &insertion);

        let Some(last_selected_index) = selected_indices.iter().max().copied() else {
            self.status_message = "Selected token no longer exists".into();
            return Ok(());
        };
        let inserted_indices: Vec<usize> =
            (last_selected_index + 1..=last_selected_index + selected_notes.len()).collect();

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_note_selection_from_indices(&inserted_indices);
        self.sync_selection_visual();
        let audition = self.audition_candidate_from_indices(&inserted_indices);
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Duplicated {} token{}",
            selected_notes.len(),
            if selected_notes.len() == 1 { "" } else { "s" }
        );
        self.audition_candidate(audition);
        Ok(())
    }

    fn duplicate_selected_bars(&mut self) -> Result<()> {
        let selected_bars = self.selected_bar_spans();
        if selected_bars.is_empty() {
            self.status_message = "d applies to bar selection only".into();
            return Ok(());
        }

        let row = selected_bars[0].row;
        if selected_bars.iter().any(|bar| bar.row != row) {
            self.status_message = "Bar duplicate currently supports one line at a time".into();
            return Ok(());
        }

        let first = selected_bars.first().cloned().unwrap();
        let last = selected_bars.last().cloned().unwrap();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(row) else {
            self.status_message = "Selected bar no longer exists".into();
            return Ok(());
        };

        let insertion = char_range(line, first.start_col + 1, last.end_col);
        insert_at_col(line, last.end_col, &insertion);

        let inserted_indices: Vec<usize> =
            (last.index + 1..=last.index + selected_bars.len()).collect();

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.restore_bar_selection_from_row_indices(row, &inserted_indices);
        self.sync_selection_visual();
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!(
            "Duplicated {} bar{}",
            selected_bars.len(),
            if selected_bars.len() == 1 { "" } else { "s" }
        );
        Ok(())
    }

    fn add_seq_line(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let insert_row = insert_row_after_cursor(&lines, cursor.0);
        lines.insert(insert_row, "seq | . . . . |".to_string());

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.textarea
            .move_cursor(CursorMove::Jump(insert_row as u16, 6));
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = "Added seq line".into();
        Ok(())
    }

    fn add_note_head_line(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let insert_row = insert_row_after_cursor(&lines, cursor.0);
        let note = self.note_token_for_add();
        lines.insert(insert_row, format!("{} | ^ . . . |", note));

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.textarea
            .move_cursor(CursorMove::Jump(insert_row as u16, 0));
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!("Added note-head line: {}", note);
        self.audition_candidate(Some((insert_row, note)));
        Ok(())
    }

    fn add_track(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let insert_row = insert_row_after_cursor(&lines, cursor.0);
        let (track_name, channel) = next_track_header(&lines);
        let inserted = vec![
            String::new(),
            format!("# {}: {}", track_name, channel),
            "seq | . . . . |".to_string(),
        ];
        lines.splice(insert_row..insert_row, inserted);

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.textarea
            .move_cursor(CursorMove::Jump((insert_row + 2) as u16, 6));
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!("Added track: {} on channel {}", track_name, channel);
        Ok(())
    }

    fn add_bar(&mut self) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(cursor.0) else {
            self.status_message = "No current line".into();
            return Ok(());
        };

        let Ok(new_cursor_col) = add_rest_bar_to_line(line) else {
            self.status_message = "Add bar needs a line with bars".into();
            return Ok(());
        };

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        self.textarea
            .move_cursor(CursorMove::Jump(cursor.0 as u16, new_cursor_col as u16));
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = "Added bar".into();
        Ok(())
    }

    fn place_token_at_current_slot(&mut self, token: &str) -> Result<()> {
        let cursor = self.textarea.cursor();
        let mut lines = self.textarea.lines().to_vec();
        let Some(line) = lines.get_mut(cursor.0) else {
            self.status_message = "No current line".into();
            return Ok(());
        };

        let Ok(slot) = place_seq_token_at_slot(cursor.0, line, cursor.1, token) else {
            self.status_message = "Place token currently supports seq lines only".into();
            return Ok(());
        };

        self.push_source_undo();
        self.replace_source(lines.join("\n"));
        if let Some(note) = self.note_spans_on_line(cursor.0).get(slot.index_on_line) {
            self.textarea
                .move_cursor(CursorMove::Jump(cursor.0 as u16, note.start_col as u16));
        }
        self.dirty = true;
        self.compile_and_update_current_source()?;
        self.status_message = format!("Placed {}", token);
        if token != "." && token != "-" {
            self.audition_candidate(Some((cursor.0, token.to_string())));
        }
        Ok(())
    }

    fn note_token_for_add(&self) -> String {
        let cursor = self.textarea.cursor();
        if let Some(note) = note_at_or_near_col(
            self.auditionable_spans_in_line(self.textarea.lines(), cursor.0),
            cursor.1,
        ) {
            return note.token;
        }

        self.textarea
            .lines()
            .iter()
            .enumerate()
            .take(cursor.0 + 1)
            .rev()
            .find_map(|(row, _)| {
                self.auditionable_spans_in_line(self.textarea.lines(), row)
                    .into_iter()
                    .next_back()
            })
            .map(|note| note.token)
            .unwrap_or_else(|| "C4".to_string())
    }

    fn apply_selected_loop_range(&mut self) -> Result<()> {
        let selected_bars = self.selected_bar_spans();
        if selected_bars.is_empty() {
            self.status_message = "Loop range applies to bar selection only".into();
            return Ok(());
        }

        let Some(start_index) = selected_bars.iter().map(|bar| bar.index).min() else {
            return Ok(());
        };
        let Some(end_index) = selected_bars.iter().map(|bar| bar.index).max() else {
            return Ok(());
        };

        let source = self.source();
        let loop_range = match loop_range_for_bar_indices(&source, start_index, end_index) {
            Ok(loop_range) => loop_range,
            Err(message) => {
                self.status_message = message;
                return Ok(());
            }
        };
        match set_loop_range_frontmatter(&source, &loop_range) {
            Ok(source) => {
                self.push_source_undo();
                self.replace_source(source);
                self.restore_bar_selection_from_row_indices(
                    selected_bars[0].row,
                    &(start_index..=end_index).collect::<Vec<_>>(),
                );
                self.sync_selection_visual();
                self.dirty = true;
                self.compile_and_update_current_source()?;
                self.status_message = format!("Loop range: {}", loop_range);
            }
            Err(message) => {
                self.status_message = message;
            }
        }
        Ok(())
    }

    fn toggle_loop(&mut self) -> Result<()> {
        match toggle_loop_frontmatter(&self.source()) {
            Ok((source, enabled)) => {
                self.push_source_undo();
                self.replace_source(source);
                self.dirty = true;
                self.compile_and_update_current_source()?;
                self.status_message = if enabled {
                    "Loop: on".into()
                } else {
                    "Loop: off".into()
                };
            }
            Err(message) => {
                self.status_message = message;
            }
        }
        Ok(())
    }

    fn clear_loop_settings(&mut self) -> Result<()> {
        match clear_loop_settings_frontmatter(&self.source()) {
            Ok(Some(source)) => {
                let cursor = self.textarea.cursor();
                self.push_source_undo();
                self.replace_source(source);
                self.textarea
                    .move_cursor(CursorMove::Jump(cursor.0 as u16, cursor.1 as u16));
                self.dirty = true;
                self.compile_and_update_current_source()?;
                self.status_message = "Loop cleared".into();
            }
            Ok(None) => {
                self.status_message = "No loop settings to clear".into();
            }
            Err(message) => {
                self.status_message = message;
            }
        }
        Ok(())
    }

    fn current_loop_range(&self) -> Option<String> {
        parser::parse_song(self.source())
            .ok()
            .and_then(|song| song.metadata.loop_range)
    }

    fn audition_candidate_from_lines(
        &self,
        lines: &[String],
        start_row: usize,
        end_row: usize,
        cursor: (usize, usize),
    ) -> Option<(usize, String)> {
        let preferred_row = if (start_row..=end_row).contains(&cursor.0) {
            cursor.0
        } else {
            start_row
        };

        note_at_or_near_col(
            self.auditionable_spans_in_line(lines, preferred_row),
            cursor.1,
        )
        .or_else(|| {
            (start_row..=end_row).find_map(|row| {
                self.auditionable_spans_in_line(lines, row)
                    .into_iter()
                    .next()
            })
        })
        .map(|note| (note.row, note.token))
    }

    fn auditionable_spans_in_line(&self, lines: &[String], row: usize) -> Vec<NoteTokenSpan> {
        lines
            .get(row)
            .map(|line| {
                note_spans_in_line(row, line)
                    .into_iter()
                    .filter(|note| note.kind == SelectableTokenKind::Note)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn audition_candidate_from_indices(&self, indices: &[usize]) -> Option<(usize, String)> {
        let notes = self.note_token_spans();
        indices.iter().find_map(|index| {
            notes
                .get(*index)
                .filter(|note| note.kind == SelectableTokenKind::Note)
                .map(|note| (note.row, note.token.clone()))
        })
    }

    fn audition_candidate(&mut self, candidate: Option<(usize, String)>) {
        let Some((row, token)) = candidate else {
            return;
        };
        self.sync_playback_state();
        if self.is_playing {
            return;
        }
        if self.preview_token(row, &token).is_some() {
            self.status_message
                .push_str(&format!(" | Audition: {}", token));
        }
    }

    fn preview_token(&self, row: usize, token: &str) -> Option<()> {
        let note = token.parse::<Note>().ok()?;
        let midi = note.to_midi_checked().ok()?;
        let channel = match note {
            Note::Drum(_) => 9,
            _ => self.track_channel_for_row(row)?,
        };
        self.player
            .preview_note(channel, midi, 96, Duration::from_millis(180));
        Some(())
    }

    fn track_channel_for_row(&self, row: usize) -> Option<u8> {
        self.textarea
            .lines()
            .iter()
            .take(row + 1)
            .rev()
            .find_map(|line| parse_track_header_channel(line))
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
            Some(StudioSelection::Bar { span }) => (span.row, span.row),
            Some(StudioSelection::BarRange { anchor, focus }) => {
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
            Some(StudioSelection::Bar { span }) => {
                format!("bar {} on line {}", span.index + 1, span.row + 1)
            }
            Some(StudioSelection::BarRange { anchor, focus }) => {
                let (start, end) = ordered_bar_span_bounds(anchor, focus);
                if start == end {
                    format!("bar {} on line {}", start.index + 1, start.row + 1)
                } else if start.row == end.row {
                    format!(
                        "bars {}..{} on line {}",
                        start.index + 1,
                        end.index + 1,
                        start.row + 1
                    )
                } else {
                    format!(
                        "bars line {}:{} to line {}:{}",
                        start.row + 1,
                        start.index + 1,
                        end.row + 1,
                        end.index + 1
                    )
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
                kind,
            }) => Some(NoteTokenSpan {
                row: *row,
                start_col: *start_col,
                end_col: *end_col,
                token: token.clone(),
                kind: *kind,
            }),
            Some(StudioSelection::NoteRange { focus, .. }) => Some(focus.clone()),
            _ => None,
        }
    }

    fn focus_bar(&self) -> Option<BarSpan> {
        match &self.selection {
            Some(StudioSelection::Bar { span }) => Some(span.clone()),
            Some(StudioSelection::BarRange { focus, .. }) => Some(focus.clone()),
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
                kind,
            }) => {
                let selected = NoteTokenSpan {
                    row: *row,
                    start_col: *start_col,
                    end_col: *end_col,
                    token: token.clone(),
                    kind: *kind,
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

    fn selected_bar_spans(&self) -> Vec<BarSpan> {
        let bars = self.bar_spans();
        match &self.selection {
            Some(StudioSelection::Bar { span }) => bars
                .iter()
                .position(|bar| bar == span)
                .and_then(|index| bars.get(index).cloned())
                .map(|bar| vec![bar])
                .unwrap_or_default(),
            Some(StudioSelection::BarRange { anchor, focus }) => {
                let Some(anchor_index) = bars.iter().position(|bar| bar == anchor) else {
                    return Vec::new();
                };
                let Some(focus_index) = bars.iter().position(|bar| bar == focus) else {
                    return Vec::new();
                };
                let start = anchor_index.min(focus_index);
                let end = anchor_index.max(focus_index);
                (start..=end)
                    .filter_map(|index| bars.get(index).cloned())
                    .collect()
            }
            _ => Vec::new(),
        }
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
                        kind: note.kind,
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

    fn restore_bar_selection_from_row_indices(&mut self, row: usize, selected_indices: &[usize]) {
        let bars = self.bar_spans_on_line(row);
        match selected_indices {
            [] => {
                self.selection = None;
            }
            [index] => {
                if let Some(bar) = bars.iter().find(|bar| bar.index == *index) {
                    self.selection = Some(StudioSelection::Bar { span: bar.clone() });
                }
            }
            indices => {
                let Some(first) = indices
                    .first()
                    .and_then(|index| bars.iter().find(|bar| bar.index == *index))
                else {
                    self.selection = None;
                    return;
                };
                let Some(last) = indices
                    .last()
                    .and_then(|index| bars.iter().find(|bar| bar.index == *index))
                else {
                    self.selection = None;
                    return;
                };
                self.selection = Some(StudioSelection::BarRange {
                    anchor: first.clone(),
                    focus: last.clone(),
                });
            }
        }
    }

    fn restore_bar_selection_from_positions(&mut self, positions: &[(usize, usize)]) {
        match positions {
            [] => {
                self.selection = None;
            }
            [(row, index)] => {
                let bars = self.bar_spans_on_line(*row);
                if let Some(bar) = bars.iter().find(|bar| bar.index == *index) {
                    self.selection = Some(StudioSelection::Bar { span: bar.clone() });
                } else {
                    self.selection = None;
                }
            }
            positions => {
                let Some((first_row, first_index)) = positions.first() else {
                    self.selection = None;
                    return;
                };
                let Some((last_row, last_index)) = positions.last() else {
                    self.selection = None;
                    return;
                };
                let first_bars = self.bar_spans_on_line(*first_row);
                let last_bars = self.bar_spans_on_line(*last_row);
                let Some(first) = first_bars.iter().find(|bar| bar.index == *first_index) else {
                    self.selection = None;
                    return;
                };
                let Some(last) = last_bars.iter().find(|bar| bar.index == *last_index) else {
                    self.selection = None;
                    return;
                };
                self.selection = Some(StudioSelection::BarRange {
                    anchor: first.clone(),
                    focus: last.clone(),
                });
            }
        }
    }

    fn note_at_or_after_cursor(&self, row: usize, col: usize) -> Option<NoteTokenSpan> {
        note_at_or_near_col(self.note_spans_on_line(row), col)
    }

    fn bar_at_or_after_cursor(&self, row: usize, col: usize) -> Option<BarSpan> {
        bar_at_or_near_col(self.bar_spans_on_line(row), col)
    }

    fn nearest_note_on_line(&self, row: usize, col: usize) -> Option<NoteTokenSpan> {
        self.note_spans_on_line(row)
            .into_iter()
            .min_by_key(|note| note.start_col.abs_diff(col))
    }

    fn nearest_bar_on_line(&self, row: usize, col: usize) -> Option<BarSpan> {
        self.bar_spans_on_line(row)
            .into_iter()
            .min_by_key(|bar| bar.start_col.abs_diff(col))
    }

    fn note_token_spans(&self) -> Vec<NoteTokenSpan> {
        self.textarea
            .lines()
            .iter()
            .enumerate()
            .flat_map(|(row, _)| self.note_spans_on_line(row))
            .collect()
    }

    fn bar_spans(&self) -> Vec<BarSpan> {
        self.textarea
            .lines()
            .iter()
            .enumerate()
            .flat_map(|(row, _)| self.bar_spans_on_line(row))
            .collect()
    }

    fn note_spans_on_line(&self, row: usize) -> Vec<NoteTokenSpan> {
        self.textarea
            .lines()
            .get(row)
            .map(|line| note_spans_in_line(row, line))
            .unwrap_or_default()
    }

    fn bar_spans_on_line(&self, row: usize) -> Vec<BarSpan> {
        self.textarea
            .lines()
            .get(row)
            .map(|line| bar_spans_in_line(row, line))
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
            StudioMode::Normal if self.pending_add => {
                "Add: s seq | l note-head | t track | b bar | n note | . rest | - sustain | Esc"
            }
            StudioMode::Normal => {
                "i ins | a add | v note | V line | b bar | B line bars | +/- transpose | space play | w save"
            }
            StudioMode::Insert => "Esc normal | type to edit | Ctrl+U undo | Ctrl+R redo",
            StudioMode::Select => {
                "h/l move | H/L extend | Enter loop | d duplicate | x delete | r rest | s sustain | Esc"
            }
        };
        let footer = Paragraph::new(help).block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);
    }
}

struct PlacedSlot {
    index_on_line: usize,
}

fn insert_row_after_cursor(lines: &[String], cursor_row: usize) -> usize {
    if lines.is_empty() {
        0
    } else {
        (cursor_row + 1).min(lines.len())
    }
}

fn next_track_header(lines: &[String]) -> (String, u8) {
    let track_count = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with('#') && !trimmed.starts_with("##")
        })
        .count();
    let next_channel = lines
        .iter()
        .filter_map(|line| parse_track_header_channel(line))
        .map(|channel| channel + 2)
        .max()
        .unwrap_or(1)
        .min(16);
    (format!("Track {}", track_count + 1), next_channel)
}

fn add_rest_bar_to_line(line: &mut String) -> std::result::Result<usize, ()> {
    if !line.contains('|') {
        return Err(());
    }
    let trimmed_len = line.trim_end().chars().count();
    replace_char_range(line, trimmed_len, line.chars().count(), "");
    let cursor_col = line.chars().count() + 1;
    line.push_str(" . . . . |");
    Ok(cursor_col)
}

fn place_seq_token_at_slot(
    row: usize,
    line: &mut String,
    col: usize,
    token: &str,
) -> std::result::Result<PlacedSlot, ()> {
    if !is_seq_line(line) {
        return Err(());
    }

    let notes = note_spans_in_line(row, line);
    if let Some((index, note)) = note_at_or_near_col_with_index(&notes, col) {
        replace_char_range(line, note.start_col, note.end_col, token);
        return Ok(PlacedSlot {
            index_on_line: index,
        });
    }

    let Some(bar) = bar_at_or_near_col(bar_spans_in_line(row, line), col) else {
        return Err(());
    };
    let insertion_col = bar.start_col + 1;
    let chars: Vec<char> = line.chars().collect();
    let insertion = if chars
        .get(insertion_col)
        .is_some_and(|ch| ch.is_whitespace())
    {
        format!(" {}", token)
    } else {
        format!(" {} ", token)
    };
    insert_at_col(line, insertion_col, &insertion);
    Ok(PlacedSlot { index_on_line: 0 })
}

fn note_at_or_near_col_with_index(
    notes: &[NoteTokenSpan],
    col: usize,
) -> Option<(usize, NoteTokenSpan)> {
    notes
        .iter()
        .enumerate()
        .find(|(_, note)| col >= note.start_col && col < note.end_col)
        .map(|(index, note)| (index, note.clone()))
        .or_else(|| {
            notes
                .iter()
                .enumerate()
                .find(|(_, note)| note.start_col >= col)
                .map(|(index, note)| (index, note.clone()))
        })
        .or_else(|| {
            notes
                .iter()
                .enumerate()
                .next_back()
                .map(|(index, note)| (index, note.clone()))
        })
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

fn transpose_bar_text(input: &str, semitones: i32) -> Result<(String, bool)> {
    transpose_note_tokens(input, semitones)
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

fn toggle_loop_frontmatter(source: &str) -> std::result::Result<(String, bool), String> {
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();

    if !matches!(lines.first().map(|line| line.as_str()), Some("---")) {
        let source = if source.is_empty() {
            "---\nloop: true\n---\n".to_string()
        } else {
            format!("---\nloop: true\n---\n\n{}", source)
        };
        return Ok((source, true));
    }

    let Some(end_index) = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line == "---").then_some(index))
    else {
        return Err("Loop toggle failed: frontmatter block is not closed".to_string());
    };

    for line in lines.iter_mut().take(end_index).skip(1) {
        let trimmed = line.trim();
        if !trimmed.starts_with("loop:") {
            continue;
        }

        match trimmed {
            "loop: true" => {
                *line = "loop: false".to_string();
                return Ok((finish_source(lines), false));
            }
            "loop: false" => {
                *line = "loop: true".to_string();
                return Ok((finish_source(lines), true));
            }
            _ => {
                return Err(
                    "Loop toggle supports only simple `loop: true` or `loop: false`".to_string(),
                );
            }
        }
    }

    lines.insert(end_index, "loop: true".to_string());
    Ok((finish_source(lines), true))
}

fn loop_range_for_bar_indices(
    source: &str,
    start_index: usize,
    end_index: usize,
) -> std::result::Result<String, String> {
    let song = parser::parse_song(source.to_string())
        .map_err(|e| format!("Cannot set loop range: {}", e))?;

    if song.metadata.unit == "beat" {
        let beats_per_bar = crate::validation::beats_per_unit("bar", &song.metadata.signature)
            .map_err(|message| format!("Cannot set loop range: {}", message))?;
        let start = start_index as f64 * beats_per_bar;
        let end = (end_index + 1) as f64 * beats_per_bar;
        Ok(format_loop_range_number(start, end))
    } else {
        Ok(format!("{}..{}", start_index, end_index + 1))
    }
}

fn format_loop_range_number(start: f64, end: f64) -> String {
    format!("{}..{}", format_loop_bound(start), format_loop_bound(end))
}

fn format_loop_bound(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{}", value as u64)
    } else {
        format!("{:.4}", value)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn set_loop_range_frontmatter(
    source: &str,
    loop_range: &str,
) -> std::result::Result<String, String> {
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();

    if !matches!(lines.first().map(|line| line.as_str()), Some("---")) {
        let source = if source.is_empty() {
            format!("---\nloop: true\nloop_range: {}\n---\n", loop_range)
        } else {
            format!(
                "---\nloop: true\nloop_range: {}\n---\n\n{}",
                loop_range, source
            )
        };
        return Ok(source);
    }

    let Some(end_index) = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line == "---").then_some(index))
    else {
        return Err("Loop range failed: frontmatter block is not closed".to_string());
    };

    let mut loop_line = None;
    let mut loop_range_line = None;
    for (index, line) in lines.iter().enumerate().take(end_index).skip(1) {
        let trimmed = line.trim();
        if trimmed.starts_with("loop:") {
            match trimmed {
                "loop: true" | "loop: false" => loop_line = Some(index),
                _ => {
                    return Err(
                        "Loop range supports only simple `loop: true` or `loop: false`".to_string(),
                    );
                }
            }
        } else if trimmed.starts_with("loop_range:") {
            if trimmed == "loop_range:" {
                return Err("Loop range supports only simple `loop_range: start..end`".to_string());
            }
            loop_range_line = Some(index);
        }
    }

    if let Some(index) = loop_line {
        lines[index] = "loop: true".to_string();
    } else {
        lines.insert(end_index, "loop: true".to_string());
        if let Some(index) = loop_range_line.as_mut() {
            *index += 1;
        }
    }

    if let Some(index) = loop_range_line {
        lines[index] = format!("loop_range: {}", loop_range);
    } else {
        let insert_index = lines
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(index, line)| (line == "---").then_some(index))
            .unwrap_or(lines.len());
        lines.insert(insert_index, format!("loop_range: {}", loop_range));
    }

    Ok(finish_source(lines))
}

fn clear_loop_settings_frontmatter(source: &str) -> std::result::Result<Option<String>, String> {
    if !matches!(source.lines().next(), Some("---")) {
        return Ok(None);
    }

    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();
    let Some(end_index) = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line == "---").then_some(index))
    else {
        return Err("Loop clear failed: frontmatter block is not closed".to_string());
    };

    let mut changed = false;
    let mut remove_indices = Vec::new();
    for (index, line) in lines.iter().enumerate().take(end_index).skip(1) {
        let trimmed = line.trim();
        if trimmed.starts_with("loop:") {
            match trimmed {
                "loop: true" => {
                    changed = true;
                    remove_indices.push(index);
                }
                "loop: false" => {
                    remove_indices.push(index);
                }
                _ => {
                    return Err(
                        "Loop clear supports only simple `loop: true` or `loop: false`".to_string(),
                    );
                }
            }
        } else if trimmed.starts_with("loop_range:") {
            if trimmed == "loop_range:" {
                return Err("Loop clear supports only simple `loop_range: start..end`".to_string());
            }
            changed = true;
            remove_indices.push(index);
        }
    }

    if remove_indices.is_empty() {
        return Ok(None);
    }

    remove_indices.sort_unstable_by(|left, right| right.cmp(left));
    for index in remove_indices {
        lines.remove(index);
    }

    if !changed {
        return Ok(None);
    }
    let Some(end_index) = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line == "---").then_some(index))
    else {
        return Err("Loop clear failed: frontmatter block is not closed".to_string());
    };
    if lines[1..end_index]
        .iter()
        .all(|line| line.trim().is_empty())
    {
        lines.drain(0..=end_index);
        if matches!(lines.first(), Some(line) if line.trim().is_empty()) {
            lines.remove(0);
        }
    }
    Ok(Some(finish_source(lines)))
}

fn parse_track_header_channel(line: &str) -> Option<u8> {
    let trimmed = line.trim();
    if trimmed.starts_with("##") || !trimmed.starts_with('#') {
        return None;
    }

    let (_, rest) = trimmed.split_once(':')?;
    let channel = rest
        .trim_start()
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>()
        .parse::<u8>()
        .ok()?;
    crate::validation::to_zero_based_channel(channel).ok()
}

fn finish_source(lines: Vec<String>) -> String {
    let mut source = lines.join("\n");
    source.push('\n');
    source
}

#[cfg(test)]
mod tests {
    use super::{
        add_rest_bar_to_line, clear_loop_settings_frontmatter, loop_range_for_bar_indices,
        next_track_header, parse_track_header_channel, place_seq_token_at_slot,
        set_loop_range_frontmatter, toggle_loop_frontmatter, transpose_bar_text, transpose_line,
    };

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
    fn transpose_bar_text_transposes_only_notes() {
        let (bar, changed) = transpose_bar_text("| D4 . - Eb4 |", 1).unwrap();
        assert!(changed);
        assert_eq!(bar, "| D#4 . - E4 |");
    }

    #[test]
    fn add_rest_bar_appends_grid_bar() {
        let mut line = "seq | C4 . |".to_string();
        let cursor_col = add_rest_bar_to_line(&mut line).unwrap();
        assert_eq!(line, "seq | C4 . | . . . . |");
        assert_eq!(cursor_col, 13);
    }

    #[test]
    fn place_seq_token_replaces_current_slot() {
        let mut line = "seq | C4 . E4 |".to_string();
        let placed = place_seq_token_at_slot(0, &mut line, 9, "D4").unwrap();
        assert_eq!(line, "seq | C4 D4 E4 |");
        assert_eq!(placed.index_on_line, 1);
    }

    #[test]
    fn place_seq_token_in_empty_bar() {
        let mut line = "seq | |".to_string();
        let placed = place_seq_token_at_slot(0, &mut line, 5, "C4").unwrap();
        assert_eq!(line, "seq | C4 |");
        assert_eq!(placed.index_on_line, 0);
    }

    #[test]
    fn next_track_header_uses_next_track_number_and_channel() {
        let lines = vec![
            "# Piano: 1".to_string(),
            "seq | C4 |".to_string(),
            "# Bass: 3".to_string(),
        ];
        assert_eq!(next_track_header(&lines), ("Track 3".to_string(), 4));
    }

    #[test]
    fn toggle_loop_adds_frontmatter_when_missing() {
        let (source, enabled) = toggle_loop_frontmatter("# Piano: 1\nC4 | ^ |\n").unwrap();
        assert!(enabled);
        assert_eq!(source, "---\nloop: true\n---\n\n# Piano: 1\nC4 | ^ |\n");
    }

    #[test]
    fn toggle_loop_adds_key_to_existing_frontmatter() {
        let (source, enabled) =
            toggle_loop_frontmatter("---\nbpm: 100\n---\n# Piano: 1\n").unwrap();
        assert!(enabled);
        assert_eq!(source, "---\nbpm: 100\nloop: true\n---\n# Piano: 1\n");
    }

    #[test]
    fn toggle_loop_turns_on_and_off() {
        let (source, enabled) =
            toggle_loop_frontmatter("---\nloop: false\n---\n# Piano: 1\n").unwrap();
        assert!(enabled);
        assert_eq!(source, "---\nloop: true\n---\n# Piano: 1\n");

        let (source, enabled) = toggle_loop_frontmatter(&source).unwrap();
        assert!(!enabled);
        assert_eq!(source, "---\nloop: false\n---\n# Piano: 1\n");
    }

    #[test]
    fn toggle_loop_rejects_non_simple_loop_value() {
        let err = toggle_loop_frontmatter("---\nloop:\n  enabled: true\n---\n").unwrap_err();
        assert_eq!(
            err,
            "Loop toggle supports only simple `loop: true` or `loop: false`"
        );
    }

    #[test]
    fn parse_track_header_channel_returns_zero_based_channel() {
        assert_eq!(parse_track_header_channel("# Piano: 2"), Some(1));
        assert_eq!(parse_track_header_channel("# Drums: 10 mute"), Some(9));
        assert_eq!(parse_track_header_channel("## sound 1"), None);
        assert_eq!(parse_track_header_channel("# Invalid: 17"), None);
    }

    #[test]
    fn set_loop_range_adds_frontmatter_when_missing() {
        let source = set_loop_range_frontmatter("# Piano: 1\nC4 | ^ |\n", "0..1").unwrap();
        assert_eq!(
            source,
            "---\nloop: true\nloop_range: 0..1\n---\n\n# Piano: 1\nC4 | ^ |\n"
        );
    }

    #[test]
    fn set_loop_range_updates_existing_scalars() {
        let source = set_loop_range_frontmatter(
            "---\nloop: false\nloop_range: 1..2\n---\n# Piano: 1\n",
            "0..4",
        )
        .unwrap();
        assert_eq!(
            source,
            "---\nloop: true\nloop_range: 0..4\n---\n# Piano: 1\n"
        );
    }

    #[test]
    fn clear_loop_settings_removes_loop_and_loop_range() {
        let source = clear_loop_settings_frontmatter(
            "---\nbpm: 100\nloop: true\nloop_range: 1..2\n---\n# Piano: 1\n",
        )
        .unwrap()
        .unwrap();

        assert_eq!(source, "---\nbpm: 100\n---\n# Piano: 1\n");
    }

    #[test]
    fn clear_loop_settings_removes_empty_frontmatter_block() {
        let source = clear_loop_settings_frontmatter(
            "---\nloop: true\nloop_range: 1..2\n---\n\n# Piano: 1\n",
        )
        .unwrap()
        .unwrap();

        assert_eq!(source, "# Piano: 1\n");
    }

    #[test]
    fn clear_loop_settings_is_noop_without_enabled_loop_settings() {
        assert_eq!(
            clear_loop_settings_frontmatter("---\nbpm: 100\nloop: false\n---\n").unwrap(),
            None
        );
        assert_eq!(
            clear_loop_settings_frontmatter("# Piano: 1\n").unwrap(),
            None
        );
    }

    #[test]
    fn loop_range_uses_bar_unit_indices() {
        let source = "---\nunit: bar\nsignature: 3/4\n---\n# Piano: 1\nC4 | ^ | ^ |\n";
        assert_eq!(loop_range_for_bar_indices(source, 1, 2).unwrap(), "1..3");
    }

    #[test]
    fn loop_range_converts_to_beats_for_beat_unit() {
        let source = "---\nunit: beat\nsignature: 3/4\n---\n# Piano: 1\nC4 | ^ | ^ |\n";
        assert_eq!(loop_range_for_bar_indices(source, 1, 2).unwrap(), "3..9");
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
