use super::input::NOTE_HELP;
use super::StudioApp;
use crate::config::NoteKeyboardConfig;
use crossterm::event::{KeyCode, KeyEvent};
use miette::Result;
use std::collections::HashMap;

const MIN_KEYBOARD_OCTAVE: i32 = 0;
const MAX_KEYBOARD_OCTAVE: i32 = 7;
const DEFAULT_KEYBOARD_OCTAVE: i32 = 4;

#[derive(Clone, Debug)]
pub(super) struct NoteKeyboard {
    keys: HashMap<char, NoteKeyBinding>,
    octave_down: char,
    octave_up: char,
}

#[derive(Clone, Debug)]
enum NoteKeyBinding {
    Pitch { name: String, octave_offset: i32 },
    Rest,
    Sustain,
}

enum NoteKeyInput {
    Cancel,
    Token(String),
    Unknown,
}

impl NoteKeyboard {
    pub(super) fn from_config(config: &NoteKeyboardConfig) -> (Self, i32) {
        let mut keyboard = Self::default();

        if let Some(key) = config.octave_down.as_deref().and_then(single_char_key) {
            keyboard.octave_down = key;
        }
        if let Some(key) = config.octave_up.as_deref().and_then(single_char_key) {
            keyboard.octave_up = key;
        }

        for (key, value) in &config.keys {
            let Some(key) = single_char_key(key) else {
                continue;
            };
            let Some(binding) = parse_note_binding(value) else {
                continue;
            };
            keyboard.keys.insert(key, binding);
        }

        let octave = config
            .base_octave
            .unwrap_or(DEFAULT_KEYBOARD_OCTAVE)
            .clamp(MIN_KEYBOARD_OCTAVE, MAX_KEYBOARD_OCTAVE);
        (keyboard, octave)
    }

    pub(super) fn is_octave_down(&self, key: char) -> bool {
        key.eq_ignore_ascii_case(&self.octave_down)
    }

    pub(super) fn is_octave_up(&self, key: char) -> bool {
        key.eq_ignore_ascii_case(&self.octave_up)
    }

    fn token(&self, key: char, base_octave: i32) -> Option<String> {
        let binding = self.keys.get(&key.to_ascii_lowercase())?;
        match binding {
            NoteKeyBinding::Pitch {
                name,
                octave_offset,
            } => Some(format!("{}{}", name, base_octave + octave_offset)),
            NoteKeyBinding::Rest => Some(".".to_string()),
            NoteKeyBinding::Sustain => Some("-".to_string()),
        }
    }
}

impl Default for NoteKeyboard {
    fn default() -> Self {
        let mut keys = HashMap::new();
        for (key, name, octave_offset) in [
            ('a', "C", 0),
            ('w', "C#", 0),
            ('s', "D", 0),
            ('e', "D#", 0),
            ('d', "E", 0),
            ('f', "F", 0),
            ('t', "F#", 0),
            ('g', "G", 0),
            ('y', "G#", 0),
            ('h', "A", 0),
            ('u', "A#", 0),
            ('j', "B", 0),
            ('k', "C", 1),
            ('o', "C#", 1),
            ('l', "D", 1),
        ] {
            keys.insert(
                key,
                NoteKeyBinding::Pitch {
                    name: name.to_string(),
                    octave_offset,
                },
            );
        }
        keys.insert('.', NoteKeyBinding::Rest);
        keys.insert('-', NoteKeyBinding::Sustain);

        Self {
            keys,
            octave_down: 'z',
            octave_up: 'x',
        }
    }
}

impl StudioApp {
    pub(super) fn handle_note_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.note_key_input(key) {
            NoteKeyInput::Cancel => {
                self.status_message = "Note entry cancelled".into();
            }
            NoteKeyInput::Token(token) => {
                self.place_token_at_current_slot(&token)?;
            }
            NoteKeyInput::Unknown => {
                self.status_message = format!("Unknown note key. {}", NOTE_HELP);
            }
        }
        Ok(())
    }

    pub(super) fn handle_select_note_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.note_key_input(key) {
            NoteKeyInput::Cancel => {
                self.status_message = "Note entry cancelled".into();
            }
            NoteKeyInput::Token(token) => {
                self.replace_selected_tokens(&token)?;
            }
            NoteKeyInput::Unknown => {
                self.status_message = format!("Unknown note key. {}", NOTE_HELP);
            }
        }
        Ok(())
    }

    pub(super) fn adjust_note_keyboard_octave(&mut self, delta: i32) {
        self.note_keyboard_octave =
            (self.note_keyboard_octave + delta).clamp(MIN_KEYBOARD_OCTAVE, MAX_KEYBOARD_OCTAVE);
        self.status_message = format!("Keyboard octave: {}", self.note_keyboard_octave);
    }

    fn note_key_input(&self, key: KeyEvent) -> NoteKeyInput {
        match key.code {
            KeyCode::Esc => NoteKeyInput::Cancel,
            KeyCode::Char(ch) => self
                .note_keyboard
                .token(ch, self.note_keyboard_octave)
                .map_or(NoteKeyInput::Unknown, NoteKeyInput::Token),
            _ => NoteKeyInput::Unknown,
        }
    }
}

fn single_char_key(value: &str) -> Option<char> {
    let mut chars = value.chars();
    let key = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    Some(key.to_ascii_lowercase())
}

fn parse_note_binding(value: &str) -> Option<NoteKeyBinding> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("rest") {
        return Some(NoteKeyBinding::Rest);
    }
    if value.eq_ignore_ascii_case("sustain") {
        return Some(NoteKeyBinding::Sustain);
    }

    let split_at = value.find(['+', '-']).unwrap_or(value.len());
    let name = &value[..split_at];
    if !is_note_name(name) {
        return None;
    }

    let octave_offset = if split_at == value.len() {
        0
    } else {
        value[split_at..].parse::<i32>().ok()?
    };

    Some(NoteKeyBinding::Pitch {
        name: name.to_string(),
        octave_offset,
    })
}

fn is_note_name(value: &str) -> bool {
    matches!(
        value,
        "C" | "C#" | "D" | "D#" | "E" | "F" | "F#" | "G" | "G#" | "A" | "A#" | "B"
    )
}

#[cfg(test)]
mod tests {
    use super::{parse_note_binding, NoteKeyboard, MAX_KEYBOARD_OCTAVE};
    use crate::config::NoteKeyboardConfig;
    use std::collections::HashMap;

    #[test]
    fn keyboard_note_maps_white_and_black_keys() {
        let keyboard = NoteKeyboard::default();

        assert_eq!(keyboard.token('a', 4).as_deref(), Some("C4"));
        assert_eq!(keyboard.token('w', 4).as_deref(), Some("C#4"));
        assert_eq!(keyboard.token('s', 4).as_deref(), Some("D4"));
        assert_eq!(keyboard.token('e', 4).as_deref(), Some("D#4"));
        assert_eq!(keyboard.token('d', 4).as_deref(), Some("E4"));
        assert_eq!(keyboard.token('k', 4).as_deref(), Some("C5"));
        assert_eq!(keyboard.token('o', 4).as_deref(), Some("C#5"));
        assert_eq!(keyboard.token('l', 4).as_deref(), Some("D5"));
        assert_eq!(keyboard.token('.', 4).as_deref(), Some("."));
        assert_eq!(keyboard.token('-', 4).as_deref(), Some("-"));
    }

    #[test]
    fn keyboard_note_accepts_uppercase_keys() {
        let keyboard = NoteKeyboard::default();

        assert_eq!(keyboard.token('A', 3).as_deref(), Some("C3"));
    }

    #[test]
    fn keyboard_note_applies_configured_overrides() {
        let mut keys = HashMap::new();
        keys.insert("r".to_string(), "C+2".to_string());
        keys.insert("a".to_string(), "G#-1".to_string());
        keys.insert(".".to_string(), "D".to_string());
        keys.insert("-".to_string(), "E".to_string());
        keys.insert("q".to_string(), "rest".to_string());
        keys.insert("w".to_string(), "sustain".to_string());
        let config = NoteKeyboardConfig {
            base_octave: Some(3),
            octave_down: Some(",".to_string()),
            octave_up: Some(".".to_string()),
            keys,
        };

        let (keyboard, octave) = NoteKeyboard::from_config(&config);

        assert_eq!(octave, 3);
        assert_eq!(keyboard.token('r', octave).as_deref(), Some("C5"));
        assert_eq!(keyboard.token('a', octave).as_deref(), Some("G#2"));
        assert_eq!(keyboard.token('.', octave).as_deref(), Some("D3"));
        assert_eq!(keyboard.token('-', octave).as_deref(), Some("E3"));
        assert_eq!(keyboard.token('q', octave).as_deref(), Some("."));
        assert_eq!(keyboard.token('w', octave).as_deref(), Some("-"));
        assert!(keyboard.is_octave_down(','));
        assert!(keyboard.is_octave_up('.'));
    }

    #[test]
    fn invalid_config_entries_fall_back_to_defaults() {
        let mut keys = HashMap::new();
        keys.insert("too-long".to_string(), "C+2".to_string());
        keys.insert("r".to_string(), "not-a-note".to_string());
        let config = NoteKeyboardConfig {
            base_octave: Some(99),
            octave_down: Some("zz".to_string()),
            octave_up: Some("xx".to_string()),
            keys,
        };

        let (keyboard, octave) = NoteKeyboard::from_config(&config);

        assert_eq!(octave, MAX_KEYBOARD_OCTAVE);
        assert_eq!(keyboard.token('a', octave).as_deref(), Some("C7"));
        assert_eq!(keyboard.token('r', octave), None);
        assert!(keyboard.is_octave_down('z'));
        assert!(keyboard.is_octave_up('x'));
    }

    #[test]
    fn note_binding_requires_supported_note_names() {
        assert!(parse_note_binding("C").is_some());
        assert!(parse_note_binding("C#+1").is_some());
        assert!(parse_note_binding("rest").is_some());
        assert!(parse_note_binding("sustain").is_some());
        assert!(parse_note_binding("Db").is_none());
        assert!(parse_note_binding("C#x").is_none());
    }
}
