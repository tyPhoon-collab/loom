use serde::Serialize;
use tabled::Tabled;

#[derive(Debug, Clone, Serialize)]
pub(super) struct ParsedEvent {
    pub event_type: String,
    pub track: String,
    pub channel: u8, // 1-based
    pub note: Option<u8>,
    pub note_name: Option<String>,
    pub velocity: Option<u8>,
    pub cc: Option<u8>,
    pub value: Option<u8>,
    pub program: Option<u8>,
    pub time: f64,
    pub duration: Option<f64>,
    pub end_time: Option<f64>,
}

#[derive(Tabled)]
pub(super) struct ParsedEventTableRow {
    #[tabled(rename = "Type")]
    pub event_type: String,
    #[tabled(rename = "Track")]
    pub track: String,
    #[tabled(rename = "CH")]
    pub channel: u8,
    #[tabled(rename = "Note")]
    pub note: String,
    #[tabled(rename = "Name")]
    pub note_name: String,
    #[tabled(rename = "Vel")]
    pub velocity: String,
    #[tabled(rename = "CC")]
    pub cc: String,
    #[tabled(rename = "Val")]
    pub value: String,
    #[tabled(rename = "PC")]
    pub program: String,
    #[tabled(rename = "Time")]
    pub time: String,
    #[tabled(rename = "Duration")]
    pub duration: String,
    #[tabled(rename = "End")]
    pub end_time: String,
}

impl From<&ParsedEvent> for ParsedEventTableRow {
    fn from(e: &ParsedEvent) -> Self {
        Self {
            event_type: e.event_type.clone(),
            track: e.track.clone(),
            channel: e.channel,
            note: e.note.map_or_else(String::new, |v| v.to_string()),
            note_name: e.note_name.clone().unwrap_or_default(),
            velocity: e.velocity.map_or_else(String::new, |v| v.to_string()),
            cc: e.cc.map_or_else(String::new, |v| v.to_string()),
            value: e.value.map_or_else(String::new, |v| v.to_string()),
            program: e.program.map_or_else(String::new, |v| v.to_string()),
            time: format!("{:.2}", e.time),
            duration: e.duration.map_or_else(String::new, |v| format!("{:.2}", v)),
            end_time: e.end_time.map_or_else(String::new, |v| format!("{:.2}", v)),
        }
    }
}

pub(super) fn midi_note_name(note: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let idx = (note % 12) as usize;
    let octave = (note / 12) as i8 - 1;
    format!("{}{}", NAMES[idx], octave)
}
