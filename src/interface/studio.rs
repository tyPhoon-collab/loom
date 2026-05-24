use crate::config::StudioConfig;
use crate::event::Event;
use crate::live_player::LivePlayer;
use crate::sequencer::PlaybackState;
use crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use input::{NoteInputMode, PendingInput, StudioInputState};
use miette::{IntoDiagnostic, Result};
use note_entry::NoteKeyboard;
use ratatui::style::{Color, Style};
use ratatui_textarea::CursorMove;
use ratatui_textarea::TextArea;
use selection::StudioSelection;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod add;
mod audition;
mod edit_ops;
mod input;
mod note_entry;
mod onset;
mod selection;
mod selection_ops;
mod selection_state;
mod selection_view;
mod settings;
mod settings_ops;
mod source;
mod template_ops;
mod track_ops;
mod transform;
mod ui;

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
    show_help_overlay: bool,
    input_state: StudioInputState,
    status_message: String,
    compile_status: CompileStatus,
    dirty: bool,
    is_playing: bool,
    bpm: u32,
    note_keyboard: NoteKeyboard,
    note_keyboard_octave: i32,
    midi_device_name: String,
    config_status: String,
    current_beat: Arc<Mutex<f64>>,
    textarea: TextArea<'static>,
    textarea_viewport: selection_view::TextAreaViewport,
    selection: Option<StudioSelection>,
    source_undo_stack: Vec<SourceUndoEntry>,
    last_continuous_edit_cursor: Option<(usize, usize)>,
    active_preview_keys: HashMap<char, ActivePreviewNote>,
    player: LivePlayer,
}

#[derive(Clone, Debug)]
struct SourceUndoEntry {
    source: String,
    cursor: (usize, usize),
}

#[derive(Clone, Copy, Debug)]
struct ActivePreviewNote {
    channel: u8,
    note: u8,
}

impl StudioApp {
    pub fn new(
        path: PathBuf,
        port_index: usize,
        config_status: String,
        studio_config: StudioConfig,
    ) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .map_err(|e| miette::miette!("Failed to read {}: {}", path.display(), e))?;
        let mut textarea = if content.is_empty() {
            TextArea::default()
        } else {
            TextArea::from(content.lines())
        };
        configure_textarea_style(&mut textarea);

        let current_beat = Arc::new(Mutex::new(0.0));
        let player = LivePlayer::new(port_index, Arc::clone(&current_beat))?;
        let midi_device_name = midi_device_name(port_index);
        let (note_keyboard, note_keyboard_octave) =
            NoteKeyboard::from_config(&studio_config.note_keyboard);

        let mut app = Self {
            should_quit: false,
            path,
            mode: StudioMode::Normal,
            show_help_overlay: false,
            input_state: StudioInputState::default(),
            status_message: "Ready".to_string(),
            compile_status: CompileStatus::Ok {
                notes: 0,
                controls: 0,
                bpm: 120,
            },
            dirty: false,
            is_playing: false,
            bpm: 120,
            note_keyboard,
            note_keyboard_octave,
            midi_device_name,
            config_status,
            current_beat,
            textarea,
            textarea_viewport: selection_view::TextAreaViewport::default(),
            selection: None,
            source_undo_stack: Vec::new(),
            last_continuous_edit_cursor: None,
            active_preview_keys: HashMap::new(),
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
                    match key.kind {
                        KeyEventKind::Press => self.handle_key(key)?,
                        KeyEventKind::Release
                            if matches!(
                                self.input_state.pending(),
                                Some(PendingInput::PreviewNote)
                            ) =>
                        {
                            self.handle_key(key)?;
                        }
                        _ => {}
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
        if self.show_help_overlay {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') => {
                    self.show_help_overlay = false;
                    self.status_message = format!(
                        "{} help closed",
                        match self.mode {
                            StudioMode::Normal => "Normal",
                            StudioMode::Insert => "Insert",
                            StudioMode::Select => "Select",
                        }
                    );
                    return Ok(());
                }
                _ => {
                    self.show_help_overlay = false;
                }
            }
        }

        match self.mode {
            StudioMode::Normal => self.handle_normal_key(key),
            StudioMode::Insert => self.handle_insert_key(key),
            StudioMode::Select => self.handle_select_key(key),
        }
    }

    fn begin_pending_input(&mut self, pending: PendingInput) {
        self.input_state.begin(pending);
        self.status_message = pending.prompt(self.note_keyboard_octave);
    }

    fn resume_continuous_input(&mut self, pending: PendingInput) {
        if !pending.is_continuous() {
            return;
        }
        self.input_state.begin(pending);
        self.status_message = pending.prompt(self.note_keyboard_octave);
    }

    fn record_continuous_edit_cursor(&mut self) {
        let cursor = self.textarea.cursor();
        self.last_continuous_edit_cursor = Some((cursor.0, cursor.1));
    }

    fn advance_after_continuous_edit(&mut self, pending: PendingInput) {
        if !pending.is_continuous() {
            return;
        }
        self.record_continuous_edit_cursor();
        let cursor = self.textarea.cursor();
        if let Some(next) = self.adjacent_unit(1, cursor.0, cursor.1) {
            self.focus_unit_cursor(&next);
        }
        self.resume_continuous_input(pending);
    }

    fn handle_continuous_input_undo(&mut self, pending: PendingInput) -> Result<bool> {
        if !pending.is_continuous() {
            return Ok(false);
        }

        let target = self.last_continuous_edit_cursor.unwrap_or_else(|| {
            let cursor = self.textarea.cursor();
            (cursor.0, cursor.1)
        });
        let undone = self.undo_last_source_edit_to(target)?;
        self.input_state.begin(pending);
        self.status_message = if undone {
            pending.prompt(self.note_keyboard_octave)
        } else {
            "Nothing to undo".into()
        };
        Ok(undone)
    }

    fn handle_normal_key(&mut self, key: KeyEvent) -> Result<()> {
        if let Some(pending) = self.input_state.take_pending() {
            return self.handle_pending_input(pending, key);
        }

        match key.code {
            KeyCode::Char('?') => {
                self.show_help_overlay = true;
                self.status_message = "Normal help".into();
            }
            _ if is_plain_char(&key, 'q') => {
                if self.dirty {
                    self.status_message = "Unsaved changes. Press w to save or Q to quit.".into();
                } else {
                    self.should_quit = true;
                }
            }
            _ if is_shift_char(&key, 'q') => {
                self.should_quit = true;
            }
            _ if is_plain_char(&key, 'i') => {
                self.mode = StudioMode::Insert;
                self.status_message = "Insert mode".into();
            }
            _ if is_plain_char(&key, 'a') => {
                self.begin_pending_input(PendingInput::Add);
            }
            _ if is_plain_char(&key, 'g') => {
                self.begin_pending_input(PendingInput::Goto);
            }
            _ if is_shift_char(&key, 'p') => {
                self.clear_active_preview_notes();
                self.begin_pending_input(PendingInput::PreviewNote);
            }
            _ if is_plain_char(&key, 'n') => {
                self.begin_pending_input(PendingInput::Note(NoteInputMode::Single));
            }
            _ if is_shift_char(&key, 'n') => {
                self.begin_pending_input(PendingInput::Note(NoteInputMode::Continuous));
            }
            _ if is_plain_char(&key, 'o') => {
                self.begin_pending_input(PendingInput::Onset(NoteInputMode::Single));
            }
            _ if is_shift_char(&key, 'o') => {
                self.begin_pending_input(PendingInput::Onset(NoteInputMode::Continuous));
            }
            _ if is_plain_char(&key, 's') => {
                self.subdivide_current_unit()?;
            }
            _ if is_shift_char(&key, 's') => {
                self.shrink_current_editable_group()?;
            }
            _ if is_plain_char(&key, 'x') => {
                self.delete_current_unit()?;
            }
            _ if is_shift_char(&key, 'd') => {
                self.begin_pending_input(PendingInput::DeleteStructure);
            }
            _ if is_plain_char(&key, 'm') => {
                self.toggle_current_track_mute()?;
            }
            _ if is_shift_char(&key, 'm') => {
                self.toggle_current_track_solo()?;
            }
            _ if is_shift_char(&key, 'x') => {
                self.clear_current_track_flags()?;
            }
            _ if is_plain_char(&key, 'v') => {
                self.enter_note_select_mode();
            }
            _ if is_shift_char(&key, 'v') => {
                self.enter_line_select_mode();
            }
            _ if is_plain_char(&key, 'b') => {
                self.enter_bar_select_mode();
            }
            _ if is_shift_char(&key, 'b') => {
                self.enter_line_bar_select_mode();
            }
            KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.clear_loop_settings()?;
            }
            _ if is_shift_char(&key, 'l') => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.clear_loop_settings()?;
                } else {
                    self.toggle_loop()?;
                }
            }
            _ if is_plain_char(&key, 'w') => {
                self.save()?;
            }
            _ if is_plain_char(&key, 'f') => {
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
            _ if is_plain_char(&key, 'r') => {
                self.player.restart();
                if !self.is_playing {
                    self.player.play();
                    self.is_playing = true;
                }
                self.status_message = "Restarted from beginning".into();
            }
            _ if is_plain_char(&key, 'u') => {
                if self.textarea.undo() {
                    self.dirty = true;
                    self.compile_and_update_current_source()?;
                } else if let Some(entry) = self.source_undo_stack.pop() {
                    self.replace_source(entry.source);
                    self.textarea.move_cursor(CursorMove::Jump(
                        entry.cursor.0 as u16,
                        entry.cursor.1 as u16,
                    ));
                    self.dirty = true;
                    self.compile_and_update_current_source()?;
                    self.status_message = "Undid transform".into();
                }
            }
            _ if is_shift_char(&key, 'r') => {
                if self.textarea.redo() {
                    self.dirty = true;
                    self.compile_and_update_current_source()?;
                }
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.apply_transpose(1);
            }
            KeyCode::Char('-') => {
                self.apply_transpose(-1);
            }
            KeyCode::Char(']') => {
                self.apply_transpose(12);
            }
            KeyCode::Char('[') => {
                self.apply_transpose(-12);
            }
            KeyCode::Char('{') => {
                self.adjust_template_call_time_scale(-1);
            }
            KeyCode::Char('}') => {
                self.adjust_template_call_time_scale(1);
            }
            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right => {
                self.textarea.input(key);
            }
            _ if is_plain_char(&key, 'j') => self.textarea.move_cursor(CursorMove::Down),
            _ if is_plain_char(&key, 'k') => self.textarea.move_cursor(CursorMove::Up),
            _ if is_plain_char(&key, 'h') => self.textarea.move_cursor(CursorMove::Back),
            _ if is_plain_char(&key, 'l') => self.textarea.move_cursor(CursorMove::Forward),
            KeyCode::Char(',') => {
                self.move_cursor_to_adjacent_unit(-1);
            }
            KeyCode::Char('.') => {
                self.move_cursor_to_adjacent_unit(1);
            }
            KeyCode::Char('<') => {
                if self.current_template_call_at_cursor().is_some() {
                    self.adjust_template_call_repeat(-1);
                } else {
                    self.move_cursor_to_adjacent_bar(-1);
                }
            }
            KeyCode::Char('>') => {
                if self.current_template_call_at_cursor().is_some() {
                    self.adjust_template_call_repeat(1);
                } else {
                    self.move_cursor_to_adjacent_bar(1);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_pending_input(&mut self, pending: PendingInput, key: KeyEvent) -> Result<()> {
        match pending {
            PendingInput::Add => self.handle_add_key(key),
            PendingInput::Goto => self.handle_goto_key(key),
            PendingInput::DeleteStructure => self.handle_delete_structure_key(key),
            PendingInput::TemplateMacro => self.handle_template_macro_key(key),
            PendingInput::PreviewNote => self.handle_preview_note_key(key),
            PendingInput::Note(mode) => self.handle_note_key(mode, key),
            PendingInput::Onset(mode) => self.handle_onset_key(mode, key),
        }
    }

    fn handle_select_key(&mut self, key: KeyEvent) -> Result<()> {
        if let Some(pending) = self.input_state.take_pending() {
            return match pending {
                PendingInput::PreviewNote => self.handle_pending_input(pending, key),
                PendingInput::Note(_) => self.handle_select_note_key(key),
                PendingInput::Onset(_) => self.handle_select_onset_key(key),
                PendingInput::Goto | PendingInput::DeleteStructure => {
                    self.handle_pending_input(pending, key)
                }
                PendingInput::Add | PendingInput::TemplateMacro => {
                    self.handle_pending_input(pending, key)
                }
            };
        }

        match key.code {
            KeyCode::Esc => {
                self.exit_select_mode();
            }
            KeyCode::Char('?') => {
                self.show_help_overlay = true;
                self.status_message = "Select help".into();
            }
            _ if is_plain_char(&key, 'n') => {
                self.begin_pending_input(PendingInput::Note(NoteInputMode::Single));
            }
            _ if is_plain_char(&key, 'o') => {
                self.begin_pending_input(PendingInput::Onset(NoteInputMode::Single));
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.apply_transpose(1);
            }
            KeyCode::Char('-') => {
                self.apply_transpose(-1);
            }
            KeyCode::Char(']') => {
                self.apply_transpose(12);
            }
            KeyCode::Char('[') => {
                self.apply_transpose(-12);
            }
            KeyCode::Char('<') => {
                self.adjust_template_call_repeat(-1);
            }
            KeyCode::Char('>') => {
                self.adjust_template_call_repeat(1);
            }
            KeyCode::Char('{') => {
                self.adjust_template_call_time_scale(-1);
            }
            KeyCode::Char('}') => {
                self.adjust_template_call_time_scale(1);
            }
            _ if is_plain_char(&key, 'x') => {
                self.delete_selection()?;
            }
            _ if is_plain_char(&key, 's') => {
                self.subdivide_selected_units()?;
            }
            _ if is_shift_char(&key, 's') => {
                self.shrink_selected_editable_groups()?;
            }
            _ if is_plain_char(&key, 'd') => {
                self.duplicate_selection()?;
            }
            _ if is_shift_char(&key, 't') => {
                self.extract_selected_bars_to_template()?;
            }
            KeyCode::Enter => {
                self.apply_selected_loop_range()?;
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.expand_select_vertical(-1)
            }
            KeyCode::Down if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.expand_select_vertical(1)
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.expand_select_horizontal(-1)
            }
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.expand_select_horizontal(1)
            }
            KeyCode::Up | KeyCode::Char('k')
                if is_plain_char(&key, 'k') || matches!(key.code, KeyCode::Up) =>
            {
                self.move_selection_vertical(-1)
            }
            KeyCode::Down | KeyCode::Char('j')
                if is_plain_char(&key, 'j') || matches!(key.code, KeyCode::Down) =>
            {
                self.move_selection_vertical(1)
            }
            KeyCode::Left | KeyCode::Char('h')
                if is_plain_char(&key, 'h') || matches!(key.code, KeyCode::Left) =>
            {
                self.move_select_horizontal(-1)
            }
            KeyCode::Right | KeyCode::Char('l')
                if is_plain_char(&key, 'l') || matches!(key.code, KeyCode::Right) =>
            {
                self.move_select_horizontal(1)
            }
            _ if is_shift_char(&key, 'k') => self.expand_select_vertical(-1),
            _ if is_shift_char(&key, 'j') => self.expand_select_vertical(1),
            _ if is_shift_char(&key, 'h') => self.expand_select_horizontal(-1),
            _ if is_shift_char(&key, 'l') => self.expand_select_horizontal(1),
            _ => {}
        }
        Ok(())
    }

    fn apply_transpose(&mut self, semitones: i32) {
        if let Err(e) = self.transpose_selection(semitones) {
            self.status_message = format!("Transpose failed: {}", e);
        }
    }

    fn adjust_template_call_repeat(&mut self, delta: i32) {
        let result = if matches!(
            self.selection,
            Some(StudioSelection::TemplateCall { .. } | StudioSelection::TemplateCallRange { .. })
        ) {
            self.adjust_selected_template_call_repeats(delta)
        } else if self.selection.is_none() && self.current_template_call_at_cursor().is_some() {
            self.adjust_current_template_call_repeat(delta)
        } else {
            return;
        };

        if let Err(e) = result {
            self.status_message = format!("Template call repeat failed: {}", e);
        }
    }

    fn adjust_template_call_time_scale(&mut self, delta: i32) {
        let result = if matches!(
            self.selection,
            Some(StudioSelection::TemplateCall { .. } | StudioSelection::TemplateCallRange { .. })
        ) {
            self.adjust_selected_template_call_time_scales(delta)
        } else if self.selection.is_none() && self.current_template_call_at_cursor().is_some() {
            self.adjust_current_template_call_time_scale(delta)
        } else {
            return;
        };

        if let Err(e) = result {
            self.status_message = format!("Template call time-scale failed: {}", e);
        }
    }

    fn handle_insert_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.code == KeyCode::Char('?') {
            self.show_help_overlay = true;
            self.status_message = "Insert help".into();
            return Ok(());
        }

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
}

pub(super) fn is_plain_char(key: &KeyEvent, ch: char) -> bool {
    matches!(key.code, KeyCode::Char(code) if code.eq_ignore_ascii_case(&ch))
        && !key.modifiers.contains(KeyModifiers::SHIFT)
}

pub(super) fn is_shift_char(key: &KeyEvent, ch: char) -> bool {
    matches!(key.code, KeyCode::Char(code) if code.eq_ignore_ascii_case(&ch))
        && key.modifiers.contains(KeyModifiers::SHIFT)
}

fn configure_textarea_style(textarea: &mut TextArea<'static>) {
    textarea.set_line_number_style(Style::default().fg(Color::DarkGray));
    textarea.set_cursor_line_style(Style::default().bg(Color::DarkGray));
    textarea.set_cursor_style(Style::default().fg(Color::Black).bg(Color::Yellow));
    textarea.set_selection_style(Style::default().bg(Color::Blue));
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
