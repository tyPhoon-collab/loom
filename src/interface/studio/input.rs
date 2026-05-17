pub(super) const ADD_HELP: &str =
    "Add: s seq | l note-head | t track | b bar | n note | . rest | - sustain";
pub(super) const NOTE_HELP: &str = "Note: keyboard piano key | . rest | - sustain | Esc cancel";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PendingInput {
    Add,
    Note,
}

#[derive(Default)]
pub(super) struct StudioInputState {
    pending: Option<PendingInput>,
}

impl StudioInputState {
    pub(super) fn begin_add(&mut self) {
        self.pending = Some(PendingInput::Add);
    }

    pub(super) fn begin_note(&mut self) {
        self.pending = Some(PendingInput::Note);
    }

    pub(super) fn take_pending(&mut self) -> Option<PendingInput> {
        self.pending.take()
    }

    pub(super) fn pending(&self) -> Option<PendingInput> {
        self.pending
    }
}
