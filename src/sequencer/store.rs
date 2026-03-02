use crate::compiler::MidiEvent;
use crate::dsl::token::Frontmatter;

#[derive(Debug, Default, Clone)]
pub struct Store {
    pub events: Vec<MidiEvent>,
    pub metadata: Frontmatter,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, events: Vec<MidiEvent>, metadata: Frontmatter) {
        self.events = events;
        self.metadata = metadata;
    }
}
