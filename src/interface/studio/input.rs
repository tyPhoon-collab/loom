pub(super) const ADD_HELP: &str =
    "Add: s seq | l note-head | t track | h separator | b bar | d drums | v velocity-mod | p pitch-mod | m template-macro | n note | . rest | - sustain";
pub(super) const GOTO_HELP: &str =
    "Goto: t next track | T previous track | d template definition | Esc cancel";
pub(super) const DELETE_HELP: &str = "Delete: t current track | Esc cancel";
pub(super) const NOTE_HELP: &str =
    "Note: keyboard piano key | . rest | - sustain | z/x octave | Esc cancel";
pub(super) const CONTINUOUS_NOTE_HELP: &str =
    "Note*: keyboard piano key | . rest | - sustain | z/x octave | Backspace undo | Esc cancel";
pub(super) const ONSET_HELP: &str = "Onset: x note-on | . rest | - sustain | t toggle | Esc cancel";
pub(super) const CONTINUOUS_ONSET_HELP: &str =
    "Onset*: x note-on | . rest | - sustain | t toggle | Backspace undo | Esc cancel";
pub(super) const TEMPLATE_MACRO_HELP: &str = "Template macro: a arp | r rev | s strum | Esc cancel";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NoteInputMode {
    Single,
    Continuous,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PendingInput {
    Add,
    Goto,
    DeleteStructure,
    TemplateMacro,
    Note(NoteInputMode),
    Onset(NoteInputMode),
}

#[derive(Default)]
pub(super) struct StudioInputState {
    pending: Option<PendingInput>,
}

impl PendingInput {
    pub(super) fn prompt(self, note_keyboard_octave: i32) -> String {
        match self {
            PendingInput::Add => ADD_HELP.to_string(),
            PendingInput::Goto => GOTO_HELP.to_string(),
            PendingInput::DeleteStructure => DELETE_HELP.to_string(),
            PendingInput::TemplateMacro => TEMPLATE_MACRO_HELP.to_string(),
            PendingInput::Note(_) => {
                format!("{} | octave {}", self.help_text(), note_keyboard_octave)
            }
            PendingInput::Onset(_) => self.help_text().to_string(),
        }
    }

    pub(super) fn help_text(self) -> &'static str {
        match self {
            PendingInput::Add => ADD_HELP,
            PendingInput::Goto => GOTO_HELP,
            PendingInput::DeleteStructure => DELETE_HELP,
            PendingInput::TemplateMacro => TEMPLATE_MACRO_HELP,
            PendingInput::Note(NoteInputMode::Single) => NOTE_HELP,
            PendingInput::Note(NoteInputMode::Continuous) => CONTINUOUS_NOTE_HELP,
            PendingInput::Onset(NoteInputMode::Single) => ONSET_HELP,
            PendingInput::Onset(NoteInputMode::Continuous) => CONTINUOUS_ONSET_HELP,
        }
    }

    pub(super) fn cancel_message(self) -> &'static str {
        match self {
            PendingInput::Add => "Add cancelled",
            PendingInput::Goto => "Goto cancelled",
            PendingInput::DeleteStructure => "Delete cancelled",
            PendingInput::TemplateMacro => "Template macro cancelled",
            PendingInput::Note(NoteInputMode::Single) => "Note entry cancelled",
            PendingInput::Note(NoteInputMode::Continuous) => "Continuous note entry cancelled",
            PendingInput::Onset(NoteInputMode::Single) => "Onset edit cancelled",
            PendingInput::Onset(NoteInputMode::Continuous) => "Continuous onset edit cancelled",
        }
    }

    pub(super) fn unknown_message(self) -> String {
        let label = match self {
            PendingInput::Add => "add command",
            PendingInput::Goto => "goto command",
            PendingInput::DeleteStructure => "delete command",
            PendingInput::TemplateMacro => "template macro command",
            PendingInput::Note(_) => "note key",
            PendingInput::Onset(_) => "onset command",
        };
        format!("Unknown {}. {}", label, self.help_text())
    }

    pub(super) fn is_continuous(self) -> bool {
        matches!(
            self,
            PendingInput::Note(NoteInputMode::Continuous)
                | PendingInput::Onset(NoteInputMode::Continuous)
        )
    }
}

impl StudioInputState {
    pub(super) fn begin(&mut self, pending: PendingInput) {
        self.pending = Some(pending);
    }

    pub(super) fn take_pending(&mut self) -> Option<PendingInput> {
        self.pending.take()
    }

    pub(super) fn pending(&self) -> Option<PendingInput> {
        self.pending
    }
}
