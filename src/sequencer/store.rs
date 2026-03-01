use crate::compiler::{MidiEvent, MidiInitEvent};
use crate::dsl::token::Frontmatter;

#[derive(Debug, Default, Clone)]
pub struct Store {
    pub note_events: Vec<MidiEvent>,
    pub init_events: Vec<MidiInitEvent>,
    pub metadata: Frontmatter,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(
        &mut self,
        note_events: Vec<MidiEvent>,
        init_events: Vec<MidiInitEvent>,
        metadata: Frontmatter,
    ) {
        self.note_events = note_events;
        self.init_events = init_events;
        self.metadata = metadata;
    }
}
