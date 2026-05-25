pub(super) const ADD_HELP: &str =
    "Add\n  s seq  l lane  t track  h separator  T template  b bar\n  d drums  v velocity  p pitch  i init  m macro  n note  . rest  - sustain";
pub(super) const GOTO_HELP: &str =
    "Goto: t next track | T previous track | d template definition | Esc cancel";
pub(super) const DELETE_HELP: &str =
    "Delete\n  s seq  l lane  t track  h separator  T template  b bar\n  v velocity  p pitch  i init  m macro";
pub(super) const NOTE_HELP: &str =
    "Note: keyboard piano key | . rest | - sustain | z/x octave | Esc cancel";
pub(super) const CONTINUOUS_NOTE_HELP: &str =
    "Note*: keyboard piano key | Space skip | . rest | - sustain | z/x octave | Tab subdivide | S-Tab shrink | Backspace undo | Esc cancel";
pub(super) const PREVIEW_NOTE_HELP: &str =
    "Preview: keyboard piano key | . rest | - sustain | z/x octave | Esc cancel";
pub(super) const ONSET_HELP: &str = "Onset: x note-on | . rest | - sustain | t toggle | Esc cancel";
pub(super) const CONTINUOUS_ONSET_HELP: &str =
    "Onset*: x note-on | Space skip | . rest | - sustain | t toggle | Tab subdivide | S-Tab shrink | Backspace undo | Esc cancel";
pub(super) const TEMPLATE_MACRO_HELP: &str = "Template macro: a arp | r rev | s strum | Esc cancel";
pub(super) const TRACK_INIT_ADD_HELP: &str =
    "Init add\n  p pc  b bank  c cc  n pan\n  v volume  e expression  m mod  s sustain";
pub(super) const TRACK_INIT_DELETE_HELP: &str =
    "Init delete\n  p pc  b bank  c cc  n pan\n  v volume  e expression  m mod  s sustain";

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
    TrackInitAdd,
    TrackInitDelete,
    PreviewNote,
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
            PendingInput::TrackInitAdd => TRACK_INIT_ADD_HELP.to_string(),
            PendingInput::TrackInitDelete => TRACK_INIT_DELETE_HELP.to_string(),
            PendingInput::PreviewNote => {
                format!("{} | octave {}", self.help_text(), note_keyboard_octave)
            }
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
            PendingInput::TrackInitAdd => TRACK_INIT_ADD_HELP,
            PendingInput::TrackInitDelete => TRACK_INIT_DELETE_HELP,
            PendingInput::PreviewNote => PREVIEW_NOTE_HELP,
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
            PendingInput::TrackInitAdd => "Track init add cancelled",
            PendingInput::TrackInitDelete => "Track init delete cancelled",
            PendingInput::PreviewNote => "Preview cancelled",
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
            PendingInput::TrackInitAdd => "track init add command",
            PendingInput::TrackInitDelete => "track init delete command",
            PendingInput::PreviewNote => "preview key",
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
