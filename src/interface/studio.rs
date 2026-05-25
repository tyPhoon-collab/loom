use crate::config::StudioConfig;
use crate::event::Event;
use crate::live_player::LivePlayer;
use crate::sequencer::PlaybackState;
use crossterm::event::{self, KeyCode, KeyEvent, KeyEventKind};
use input::{NoteInputMode, PendingInput, StudioInputState};
use keystroke::{key_stroke_matches, lookup_key_action, KeyBinding, KeyStroke};
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
mod keystroke;
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

#[derive(Clone, Copy, Debug)]
enum KeyAction {
    Quit,
    ForceQuit,
    EnterInsertMode,
    BeginPending(PendingInput),
    ClearPreviewAndBegin(PendingInput),
    SubdivideCurrentUnit,
    ShrinkCurrentEditableGroup,
    DeleteCurrentUnit,
    ToggleCurrentTrackMute,
    ToggleCurrentTrackSolo,
    ClearCurrentTrackFlags,
    EnterNoteSelectMode,
    EnterLineSelectMode,
    EnterBarSelectMode,
    EnterLineBarSelectMode,
    ClearLoopSettings,
    ToggleLoop,
    Save,
    FormatCurrentSource,
    TogglePlayback,
    RestartPlayback,
    Undo,
    Redo,
    ExitSelectMode,
    DeleteSelection,
    SubdivideSelectedUnits,
    ShrinkSelectedEditableGroups,
    DuplicateSelection,
    ExtractSelectedBarsToTemplate,
    ApplySelectedLoopRange,
    ExpandSelectVertical(i32),
    ExpandSelectHorizontal(i32),
    MoveSelectionVertical(i32),
    MoveSelectionHorizontal(i32),
}

#[derive(Clone, Copy, Debug)]
enum CursorMotion {
    Up,
    Down,
    Back,
    Forward,
}

#[derive(Clone, Copy, Debug)]
enum NormalFallbackAction {
    Transpose(i32),
    AdjustTemplateCallTimeScale(i32),
    MoveCursor(CursorMotion),
    MoveAdjacentBarOrRepeat(i32),
    MoveAdjacentUnit(i32),
    PassThroughTextArea,
}

#[derive(Clone, Copy, Debug)]
enum SelectFallbackAction {
    Transpose(i32),
    AdjustTemplateCallTimeScale(i32),
    AdjustTemplateCallRepeat(i32),
}

const NORMAL_KEY_BINDINGS: &[KeyBinding<KeyAction>] = &[
    KeyBinding {
        stroke: KeyStroke::Char('q'),
        action: KeyAction::Quit,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('q'),
        action: KeyAction::ForceQuit,
    },
    KeyBinding {
        stroke: KeyStroke::Char('i'),
        action: KeyAction::EnterInsertMode,
    },
    KeyBinding {
        stroke: KeyStroke::Char('a'),
        action: KeyAction::BeginPending(PendingInput::Add),
    },
    KeyBinding {
        stroke: KeyStroke::Char('g'),
        action: KeyAction::BeginPending(PendingInput::Goto),
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('p'),
        action: KeyAction::ClearPreviewAndBegin(PendingInput::PreviewNote),
    },
    KeyBinding {
        stroke: KeyStroke::Char('n'),
        action: KeyAction::BeginPending(PendingInput::Note(NoteInputMode::Single)),
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('n'),
        action: KeyAction::BeginPending(PendingInput::Note(NoteInputMode::Continuous)),
    },
    KeyBinding {
        stroke: KeyStroke::Char('o'),
        action: KeyAction::BeginPending(PendingInput::Onset(NoteInputMode::Single)),
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('o'),
        action: KeyAction::BeginPending(PendingInput::Onset(NoteInputMode::Continuous)),
    },
    KeyBinding {
        stroke: KeyStroke::Char('s'),
        action: KeyAction::SubdivideCurrentUnit,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('s'),
        action: KeyAction::ShrinkCurrentEditableGroup,
    },
    KeyBinding {
        stroke: KeyStroke::Char('x'),
        action: KeyAction::DeleteCurrentUnit,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('d'),
        action: KeyAction::BeginPending(PendingInput::DeleteStructure),
    },
    KeyBinding {
        stroke: KeyStroke::Char('m'),
        action: KeyAction::ToggleCurrentTrackMute,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('m'),
        action: KeyAction::ToggleCurrentTrackSolo,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('x'),
        action: KeyAction::ClearCurrentTrackFlags,
    },
    KeyBinding {
        stroke: KeyStroke::Char('v'),
        action: KeyAction::EnterNoteSelectMode,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('v'),
        action: KeyAction::EnterLineSelectMode,
    },
    KeyBinding {
        stroke: KeyStroke::Char('b'),
        action: KeyAction::EnterBarSelectMode,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('b'),
        action: KeyAction::EnterLineBarSelectMode,
    },
    KeyBinding {
        stroke: KeyStroke::CtrlChar('l'),
        action: KeyAction::ClearLoopSettings,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('l'),
        action: KeyAction::ToggleLoop,
    },
    KeyBinding {
        stroke: KeyStroke::Char('w'),
        action: KeyAction::Save,
    },
    KeyBinding {
        stroke: KeyStroke::Char('f'),
        action: KeyAction::FormatCurrentSource,
    },
    KeyBinding {
        stroke: KeyStroke::Char(' '),
        action: KeyAction::TogglePlayback,
    },
    KeyBinding {
        stroke: KeyStroke::Char('r'),
        action: KeyAction::RestartPlayback,
    },
    KeyBinding {
        stroke: KeyStroke::Char('u'),
        action: KeyAction::Undo,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('r'),
        action: KeyAction::Redo,
    },
];

const SELECT_KEY_BINDINGS: &[KeyBinding<KeyAction>] = &[
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Esc),
        action: KeyAction::ExitSelectMode,
    },
    KeyBinding {
        stroke: KeyStroke::Char('n'),
        action: KeyAction::BeginPending(PendingInput::Note(NoteInputMode::Single)),
    },
    KeyBinding {
        stroke: KeyStroke::Char('o'),
        action: KeyAction::BeginPending(PendingInput::Onset(NoteInputMode::Single)),
    },
    KeyBinding {
        stroke: KeyStroke::Char('x'),
        action: KeyAction::DeleteSelection,
    },
    KeyBinding {
        stroke: KeyStroke::Char('s'),
        action: KeyAction::SubdivideSelectedUnits,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('s'),
        action: KeyAction::ShrinkSelectedEditableGroups,
    },
    KeyBinding {
        stroke: KeyStroke::Char('d'),
        action: KeyAction::DuplicateSelection,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('t'),
        action: KeyAction::ExtractSelectedBarsToTemplate,
    },
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Enter),
        action: KeyAction::ApplySelectedLoopRange,
    },
    KeyBinding {
        stroke: KeyStroke::ShiftCode(KeyCode::Up),
        action: KeyAction::ExpandSelectVertical(-1),
    },
    KeyBinding {
        stroke: KeyStroke::ShiftCode(KeyCode::Down),
        action: KeyAction::ExpandSelectVertical(1),
    },
    KeyBinding {
        stroke: KeyStroke::ShiftCode(KeyCode::Left),
        action: KeyAction::ExpandSelectHorizontal(-1),
    },
    KeyBinding {
        stroke: KeyStroke::ShiftCode(KeyCode::Right),
        action: KeyAction::ExpandSelectHorizontal(1),
    },
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Up),
        action: KeyAction::MoveSelectionVertical(-1),
    },
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Down),
        action: KeyAction::MoveSelectionVertical(1),
    },
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Left),
        action: KeyAction::MoveSelectionHorizontal(-1),
    },
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Right),
        action: KeyAction::MoveSelectionHorizontal(1),
    },
    KeyBinding {
        stroke: KeyStroke::Char('k'),
        action: KeyAction::MoveSelectionVertical(-1),
    },
    KeyBinding {
        stroke: KeyStroke::Char('j'),
        action: KeyAction::MoveSelectionVertical(1),
    },
    KeyBinding {
        stroke: KeyStroke::Char('h'),
        action: KeyAction::MoveSelectionHorizontal(-1),
    },
    KeyBinding {
        stroke: KeyStroke::Char('l'),
        action: KeyAction::MoveSelectionHorizontal(1),
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('k'),
        action: KeyAction::ExpandSelectVertical(-1),
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('j'),
        action: KeyAction::ExpandSelectVertical(1),
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('h'),
        action: KeyAction::ExpandSelectHorizontal(-1),
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('l'),
        action: KeyAction::ExpandSelectHorizontal(1),
    },
];

const NORMAL_FALLBACK_BINDINGS: &[KeyBinding<NormalFallbackAction>] = &[
    KeyBinding {
        stroke: KeyStroke::Symbol('+'),
        action: NormalFallbackAction::Transpose(1),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('='),
        action: NormalFallbackAction::Transpose(1),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('-'),
        action: NormalFallbackAction::Transpose(-1),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol(']'),
        action: NormalFallbackAction::Transpose(12),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('['),
        action: NormalFallbackAction::Transpose(-12),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('{'),
        action: NormalFallbackAction::AdjustTemplateCallTimeScale(-1),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('}'),
        action: NormalFallbackAction::AdjustTemplateCallTimeScale(1),
    },
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Up),
        action: NormalFallbackAction::PassThroughTextArea,
    },
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Down),
        action: NormalFallbackAction::PassThroughTextArea,
    },
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Left),
        action: NormalFallbackAction::PassThroughTextArea,
    },
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Right),
        action: NormalFallbackAction::PassThroughTextArea,
    },
    KeyBinding {
        stroke: KeyStroke::Char('j'),
        action: NormalFallbackAction::MoveCursor(CursorMotion::Down),
    },
    KeyBinding {
        stroke: KeyStroke::Char('k'),
        action: NormalFallbackAction::MoveCursor(CursorMotion::Up),
    },
    KeyBinding {
        stroke: KeyStroke::Char('h'),
        action: NormalFallbackAction::MoveCursor(CursorMotion::Back),
    },
    KeyBinding {
        stroke: KeyStroke::Char('l'),
        action: NormalFallbackAction::MoveCursor(CursorMotion::Forward),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('<'),
        action: NormalFallbackAction::MoveAdjacentBarOrRepeat(-1),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('>'),
        action: NormalFallbackAction::MoveAdjacentBarOrRepeat(1),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol(','),
        action: NormalFallbackAction::MoveAdjacentUnit(-1),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('.'),
        action: NormalFallbackAction::MoveAdjacentUnit(1),
    },
];

const SELECT_FALLBACK_BINDINGS: &[KeyBinding<SelectFallbackAction>] = &[
    KeyBinding {
        stroke: KeyStroke::Symbol('+'),
        action: SelectFallbackAction::Transpose(1),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('='),
        action: SelectFallbackAction::Transpose(1),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('-'),
        action: SelectFallbackAction::Transpose(-1),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol(']'),
        action: SelectFallbackAction::Transpose(12),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('['),
        action: SelectFallbackAction::Transpose(-12),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('<'),
        action: SelectFallbackAction::AdjustTemplateCallRepeat(-1),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('>'),
        action: SelectFallbackAction::AdjustTemplateCallRepeat(1),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('{'),
        action: SelectFallbackAction::AdjustTemplateCallTimeScale(-1),
    },
    KeyBinding {
        stroke: KeyStroke::Symbol('}'),
        action: SelectFallbackAction::AdjustTemplateCallTimeScale(1),
    },
];

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
    continuous_input_history: Vec<ContinuousInputStep>,
    active_preview_keys: HashMap<char, ActivePreviewNote>,
    player: LivePlayer,
}

#[derive(Clone, Debug)]
struct SourceUndoEntry {
    source: String,
    cursor: (usize, usize),
}

#[derive(Clone, Copy, Debug)]
enum ContinuousInputStep {
    Edit((usize, usize)),
    Skip((usize, usize)),
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
            continuous_input_history: Vec::new(),
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
                        KeyEventKind::Press | KeyEventKind::Repeat => self.handle_key(key)?,
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
            if is_escape_key(&key) || is_help_key(&key) {
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
            self.show_help_overlay = false;
        }

        if is_help_key(&key) {
            self.show_help_overlay = true;
            self.status_message = format!(
                "{} help",
                match self.mode {
                    StudioMode::Normal => "Normal",
                    StudioMode::Insert => "Insert",
                    StudioMode::Select => "Select",
                }
            );
            return Ok(());
        }

        match self.mode {
            StudioMode::Normal => self.handle_normal_key(key),
            StudioMode::Insert => self.handle_insert_key(key),
            StudioMode::Select => self.handle_select_key(key),
        }
    }

    fn begin_pending_input(&mut self, pending: PendingInput) {
        if pending.is_continuous() {
            self.continuous_input_history.clear();
        }
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

    fn record_continuous_step(&mut self, step: ContinuousInputStep) {
        const MAX_CONTINUOUS_INPUT_HISTORY: usize = 128;
        self.continuous_input_history.push(step);
        if self.continuous_input_history.len() > MAX_CONTINUOUS_INPUT_HISTORY {
            self.continuous_input_history.remove(0);
        }
    }

    fn record_continuous_edit_cursor(&mut self) {
        let cursor = self.textarea.cursor();
        self.record_continuous_step(ContinuousInputStep::Edit((cursor.0, cursor.1)));
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

    fn skip_current_continuous_input(&mut self, pending: PendingInput) {
        if !pending.is_continuous() {
            return;
        }
        let cursor = self.textarea.cursor();
        self.record_continuous_step(ContinuousInputStep::Skip((cursor.0, cursor.1)));
        if let Some(next) = self.adjacent_unit(1, cursor.0, cursor.1) {
            self.focus_unit_cursor(&next);
        }
        self.resume_continuous_input(pending);
    }

    fn handle_continuous_input_undo(&mut self, pending: PendingInput) -> Result<bool> {
        if !pending.is_continuous() {
            return Ok(false);
        }

        let Some(step) = self.continuous_input_history.pop() else {
            self.input_state.begin(pending);
            self.status_message = "Nothing to undo".into();
            return Ok(false);
        };

        let undone = match step {
            ContinuousInputStep::Edit(target) => self.undo_last_source_edit_to(target)?,
            ContinuousInputStep::Skip(target) => {
                if let Some(token) = self.unit_at_or_after_cursor(target.0, target.1) {
                    self.focus_unit_cursor(&token);
                } else {
                    self.textarea
                        .move_cursor(CursorMove::Jump(target.0 as u16, target.1 as u16));
                }
                true
            }
        };
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

        if let Some(action) = lookup_key_action(NORMAL_KEY_BINDINGS, &key) {
            return self.execute_key_action(action);
        }

        if let Some(action) = lookup_key_action(NORMAL_FALLBACK_BINDINGS, &key) {
            match action {
                NormalFallbackAction::Transpose(semitones) => self.apply_transpose(semitones),
                NormalFallbackAction::AdjustTemplateCallTimeScale(delta) => {
                    self.adjust_template_call_time_scale(delta);
                }
                NormalFallbackAction::MoveCursor(motion) => self.move_cursor(motion),
                NormalFallbackAction::MoveAdjacentBarOrRepeat(delta) => {
                    if self.current_template_call_at_cursor().is_some() {
                        self.adjust_template_call_repeat(delta);
                    } else {
                        self.move_cursor_to_adjacent_bar(delta);
                    }
                }
                NormalFallbackAction::MoveAdjacentUnit(delta) => {
                    self.move_cursor_to_adjacent_unit(delta);
                }
                NormalFallbackAction::PassThroughTextArea => {
                    self.textarea.input(key);
                }
            }
        }
        Ok(())
    }

    fn handle_pending_input(&mut self, pending: PendingInput, key: KeyEvent) -> Result<()> {
        match pending {
            PendingInput::Add => self.handle_add_key(key),
            PendingInput::Goto => self.handle_goto_key(key),
            PendingInput::DeleteStructure => self.handle_delete_structure_key(key),
            PendingInput::TemplateMacro => self.handle_template_macro_key(key),
            PendingInput::TrackInitAdd => self.handle_track_init_add_key(key),
            PendingInput::TrackInitDelete => self.handle_track_init_delete_key(key),
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
                PendingInput::Add
                | PendingInput::TemplateMacro
                | PendingInput::TrackInitAdd
                | PendingInput::TrackInitDelete => self.handle_pending_input(pending, key),
            };
        }

        if let Some(action) = lookup_key_action(SELECT_KEY_BINDINGS, &key) {
            return self.execute_key_action(action);
        }

        if let Some(action) = lookup_key_action(SELECT_FALLBACK_BINDINGS, &key) {
            match action {
                SelectFallbackAction::Transpose(semitones) => self.apply_transpose(semitones),
                SelectFallbackAction::AdjustTemplateCallTimeScale(delta) => {
                    self.adjust_template_call_time_scale(delta);
                }
                SelectFallbackAction::AdjustTemplateCallRepeat(delta) => {
                    self.adjust_template_call_repeat(delta);
                }
            }
        }
        Ok(())
    }

    fn execute_key_action(&mut self, action: KeyAction) -> Result<()> {
        match action {
            KeyAction::Quit => {
                if self.dirty {
                    self.status_message = "Unsaved changes. Press w to save or Q to quit.".into();
                } else {
                    self.should_quit = true;
                }
            }
            KeyAction::ForceQuit => {
                self.should_quit = true;
            }
            KeyAction::EnterInsertMode => {
                self.mode = StudioMode::Insert;
                self.status_message = "Insert mode".into();
            }
            KeyAction::BeginPending(pending) => self.begin_pending_input(pending),
            KeyAction::ClearPreviewAndBegin(pending) => {
                self.clear_active_preview_notes();
                self.begin_pending_input(pending);
            }
            KeyAction::SubdivideCurrentUnit => self.subdivide_current_unit()?,
            KeyAction::ShrinkCurrentEditableGroup => self.shrink_current_editable_group()?,
            KeyAction::DeleteCurrentUnit => self.delete_current_unit()?,
            KeyAction::ToggleCurrentTrackMute => self.toggle_current_track_mute()?,
            KeyAction::ToggleCurrentTrackSolo => self.toggle_current_track_solo()?,
            KeyAction::ClearCurrentTrackFlags => self.clear_current_track_flags()?,
            KeyAction::EnterNoteSelectMode => self.enter_note_select_mode(),
            KeyAction::EnterLineSelectMode => self.enter_line_select_mode(),
            KeyAction::EnterBarSelectMode => self.enter_bar_select_mode(),
            KeyAction::EnterLineBarSelectMode => self.enter_line_bar_select_mode(),
            KeyAction::ClearLoopSettings => self.clear_loop_settings()?,
            KeyAction::ToggleLoop => self.toggle_loop()?,
            KeyAction::Save => self.save()?,
            KeyAction::FormatCurrentSource => self.format_current_source()?,
            KeyAction::TogglePlayback => {
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
            KeyAction::RestartPlayback => {
                self.player.restart();
                if !self.is_playing {
                    self.player.play();
                    self.is_playing = true;
                }
                self.status_message = "Restarted from beginning".into();
            }
            KeyAction::Undo => {
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
            KeyAction::Redo => {
                if self.textarea.redo() {
                    self.dirty = true;
                    self.compile_and_update_current_source()?;
                }
            }
            KeyAction::ExitSelectMode => self.exit_select_mode(),
            KeyAction::DeleteSelection => self.delete_selection()?,
            KeyAction::SubdivideSelectedUnits => self.subdivide_selected_units()?,
            KeyAction::ShrinkSelectedEditableGroups => self.shrink_selected_editable_groups()?,
            KeyAction::DuplicateSelection => self.duplicate_selection()?,
            KeyAction::ExtractSelectedBarsToTemplate => self.extract_selected_bars_to_template()?,
            KeyAction::ApplySelectedLoopRange => self.apply_selected_loop_range()?,
            KeyAction::ExpandSelectVertical(delta) => self.expand_select_vertical(delta),
            KeyAction::ExpandSelectHorizontal(delta) => self.expand_select_horizontal(delta),
            KeyAction::MoveSelectionVertical(delta) => self.move_selection_vertical(delta),
            KeyAction::MoveSelectionHorizontal(delta) => self.move_select_horizontal(delta),
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
        if is_escape_key(&key) {
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

    fn move_cursor(&mut self, motion: CursorMotion) {
        let motion = match motion {
            CursorMotion::Up => CursorMove::Up,
            CursorMotion::Down => CursorMove::Down,
            CursorMotion::Back => CursorMove::Back,
            CursorMotion::Forward => CursorMove::Forward,
        };
        self.textarea.move_cursor(motion);
    }
}

fn is_escape_key(key: &KeyEvent) -> bool {
    key_stroke_matches(KeyStroke::Code(KeyCode::Esc), key)
}

fn is_help_key(key: &KeyEvent) -> bool {
    key_stroke_matches(KeyStroke::Symbol('?'), key)
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
