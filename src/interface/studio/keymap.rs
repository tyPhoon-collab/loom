use super::input::{NoteInputMode, PendingInput};
use super::keystroke::{KeyBinding, KeyStroke};
use crossterm::event::KeyCode;

#[derive(Clone, Copy, Debug)]
pub(super) enum KeyAction {
    Quit,
    ForceQuit,
    EnterInsertMode,
    EnterCommandMode,
    BeginPending(PendingInput),
    TogglePreviewPanel,
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
    Save,
    FormatCurrentSource,
    TogglePlayback,
    RestartPlayback,
    NavigateBack,
    Undo,
    Redo,
    ExitSelectMode,
    DeleteSelection,
    YankSelection,
    PasteAfter,
    SubdivideSelectedUnits,
    ShrinkSelectedEditableGroups,
    ExtractSelectedBarsToTemplate,
    ApplySelectedLoopRange,
    ExpandSelectVertical(i32),
    ExpandSelectHorizontal(i32),
    MoveSelectionVertical(i32),
    MoveSelectionHorizontal(i32),
}

#[derive(Clone, Copy, Debug)]
pub(super) enum CursorMotion {
    Up,
    Down,
    Back,
    Forward,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum NormalFallbackAction {
    Transpose(i32),
    AdjustTemplateCallTimeScale(i32),
    MoveCursor(CursorMotion),
    MoveAdjacentBarOrRepeat(i32),
    MoveAdjacentUnit(i32),
    PassThroughTextArea,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum SelectFallbackAction {
    Transpose(i32),
    AdjustTemplateCallTimeScale(i32),
    AdjustTemplateCallRepeat(i32),
}

pub(super) const NORMAL_KEY_BINDINGS: &[KeyBinding<KeyAction>] = &[
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
        stroke: KeyStroke::Symbol(':'),
        action: KeyAction::EnterCommandMode,
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
        stroke: KeyStroke::Char('c'),
        action: KeyAction::BeginPending(PendingInput::Change),
    },
    KeyBinding {
        stroke: KeyStroke::ShiftChar('p'),
        action: KeyAction::TogglePreviewPanel,
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
        stroke: KeyStroke::Char('d'),
        action: KeyAction::BeginPending(PendingInput::DeleteStructure),
    },
    KeyBinding {
        stroke: KeyStroke::Char('p'),
        action: KeyAction::PasteAfter,
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
        stroke: KeyStroke::CtrlChar('o'),
        action: KeyAction::NavigateBack,
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

pub(super) const SELECT_KEY_BINDINGS: &[KeyBinding<KeyAction>] = &[
    KeyBinding {
        stroke: KeyStroke::Code(KeyCode::Esc),
        action: KeyAction::ExitSelectMode,
    },
    KeyBinding {
        stroke: KeyStroke::Symbol(':'),
        action: KeyAction::EnterCommandMode,
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
        stroke: KeyStroke::Char('d'),
        action: KeyAction::DeleteSelection,
    },
    KeyBinding {
        stroke: KeyStroke::Char('x'),
        action: KeyAction::DeleteSelection,
    },
    KeyBinding {
        stroke: KeyStroke::Char('y'),
        action: KeyAction::YankSelection,
    },
    KeyBinding {
        stroke: KeyStroke::Char('p'),
        action: KeyAction::PasteAfter,
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

pub(super) const NORMAL_FALLBACK_BINDINGS: &[KeyBinding<NormalFallbackAction>] = &[
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

pub(super) const SELECT_FALLBACK_BINDINGS: &[KeyBinding<SelectFallbackAction>] = &[
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
