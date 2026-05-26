use super::input::{NoteInputMode, PendingInput, NOTE_HELP};
use super::keystroke::{key_stroke_matches, normalized_key_stroke, KeyStroke};
use super::StudioApp;
use crate::config::NoteKeyboardConfig;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreviewAction {
    AuditionPitch,
    SilentToken,
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
    pub(super) fn handle_preview_note_key(&mut self, key: KeyEvent) -> Result<()> {
        let pending = PendingInput::PreviewNote;

        if key.kind == KeyEventKind::Release {
            if let Some(ch) = preview_key_char(&key) {
                if let Some(active) = self.active_preview_keys.remove(&ch) {
                    self.player.preview_note_off(active.channel, active.note);
                }
            }
            self.retain_pending_with_prompt(pending);
            return Ok(());
        }

        match note_keyboard_action(&self.note_keyboard, &key) {
            Some(NoteKeyboardAction::OctaveDown) => {
                self.adjust_note_keyboard_octave(-1);
                self.retain_pending_with_prompt(pending);
                return Ok(());
            }
            Some(NoteKeyboardAction::OctaveUp) => {
                self.adjust_note_keyboard_octave(1);
                self.retain_pending_with_prompt(pending);
                return Ok(());
            }
            None => {}
        }

        let Some(ch) = preview_key_char(&key) else {
            match self.note_key_input(key) {
                NoteKeyInput::Cancel => {
                    self.clear_active_preview_notes();
                    self.cancel_pending_input(pending);
                }
                NoteKeyInput::Unknown => self.reject_pending_input(pending),
                NoteKeyInput::Token(_) => unreachable!(),
            }
            return Ok(());
        };

        if self.active_preview_keys.contains_key(&ch) {
            self.retain_pending_with_prompt(pending);
            return Ok(());
        }

        match preview_action(self.note_key_input(key)) {
            Some(PreviewAction::AuditionPitch) => {
                let NoteKeyInput::Token(token) = self.note_key_input(key) else {
                    unreachable!();
                };
                let row = self.textarea.cursor().0;
                if self.is_playing {
                    self.status_message = format!("Preview suppressed while playing: {}", token);
                } else if let Some((channel, note)) = self.preview_target(row, &token) {
                    self.player.preview_note_on(channel, note, 96);
                    self.active_preview_keys
                        .insert(ch, super::ActivePreviewNote { channel, note });
                    self.status_message = format!("Preview: {}", token);
                } else {
                    self.active_preview_keys.remove(&ch);
                    self.status_message = format!("Preview unavailable here: {}", token);
                }
                self.retain_pending_with_prompt(pending);
            }
            Some(PreviewAction::SilentToken) => {
                let NoteKeyInput::Token(token) = self.note_key_input(key) else {
                    unreachable!();
                };
                self.active_preview_keys.remove(&ch);
                self.status_message = format!("Preview silent: {}", token);
                self.retain_pending_with_prompt(pending);
            }
            None => match self.note_key_input(key) {
                NoteKeyInput::Cancel => self.cancel_pending_input(pending),
                NoteKeyInput::Unknown => self.reject_pending_input(pending),
                NoteKeyInput::Token(_) => unreachable!(),
            },
        }
        Ok(())
    }

    pub(super) fn handle_note_key(&mut self, mode: NoteInputMode, key: KeyEvent) -> Result<()> {
        let pending = PendingInput::Note(mode);

        match normalized_key_stroke(&key) {
            Some(KeyStroke::Char(' ')) if matches!(mode, NoteInputMode::Continuous) => {
                self.skip_current_continuous_input(pending);
                return Ok(());
            }
            Some(KeyStroke::Code(KeyCode::Tab)) if matches!(mode, NoteInputMode::Continuous) => {
                self.subdivide_current_unit()?;
                self.resume_continuous_input(pending);
                return Ok(());
            }
            Some(KeyStroke::Code(KeyCode::BackTab))
                if matches!(mode, NoteInputMode::Continuous) =>
            {
                self.shrink_current_editable_group()?;
                self.resume_continuous_input(pending);
                return Ok(());
            }
            Some(KeyStroke::Code(KeyCode::Backspace))
                if matches!(mode, NoteInputMode::Continuous) =>
            {
                self.handle_continuous_input_undo(pending)?;
                return Ok(());
            }
            Some(KeyStroke::Char(_)) | Some(KeyStroke::ShiftChar(_)) => {}
            _ => {}
        }

        match note_keyboard_action(&self.note_keyboard, &key) {
            Some(NoteKeyboardAction::OctaveDown) => {
                self.adjust_note_keyboard_octave(-1);
                if pending.is_continuous() {
                    self.resume_continuous_input(pending);
                }
                return Ok(());
            }
            Some(NoteKeyboardAction::OctaveUp) => {
                self.adjust_note_keyboard_octave(1);
                if pending.is_continuous() {
                    self.resume_continuous_input(pending);
                }
                return Ok(());
            }
            None => {}
        }

        match self.note_key_input(key) {
            NoteKeyInput::Cancel => {
                self.cancel_pending_input(pending);
            }
            NoteKeyInput::Token(token) => {
                let placed = self.place_token_at_current_slot(&token)?;
                if pending.is_continuous() {
                    if placed {
                        self.advance_after_continuous_edit(pending);
                    } else {
                        self.resume_continuous_input(pending);
                    }
                }
            }
            NoteKeyInput::Unknown => {
                if pending.is_continuous() {
                    self.reject_pending_input(pending);
                } else {
                    self.status_message = pending.unknown_message();
                }
            }
        }
        Ok(())
    }

    pub(super) fn handle_select_note_key(&mut self, key: KeyEvent) -> Result<()> {
        match note_keyboard_action(&self.note_keyboard, &key) {
            Some(NoteKeyboardAction::OctaveDown) => {
                self.adjust_note_keyboard_octave(-1);
                return Ok(());
            }
            Some(NoteKeyboardAction::OctaveUp) => {
                self.adjust_note_keyboard_octave(1);
                return Ok(());
            }
            None => {}
        }

        match self.note_key_input(key) {
            NoteKeyInput::Cancel => {
                self.status_message = "Note entry cancelled".into();
            }
            NoteKeyInput::Token(token) => {
                self.replace_selected_units(&token)?;
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
        note_key_input_for_key(&self.note_keyboard, self.note_keyboard_octave, key)
    }
}

fn note_key_input_for_key(keyboard: &NoteKeyboard, octave: i32, key: KeyEvent) -> NoteKeyInput {
    if key_stroke_matches(KeyStroke::Code(KeyCode::Esc), &key) {
        return NoteKeyInput::Cancel;
    }

    match normalized_key_stroke(&key) {
        Some(KeyStroke::Char(ch)) | Some(KeyStroke::ShiftChar(ch)) => keyboard
            .token(ch, octave)
            .map_or(NoteKeyInput::Unknown, NoteKeyInput::Token),
        Some(KeyStroke::Symbol(ch)) => keyboard
            .token(ch, octave)
            .map_or(NoteKeyInput::Unknown, NoteKeyInput::Token),
        _ => NoteKeyInput::Unknown,
    }
}

fn preview_action(input: NoteKeyInput) -> Option<PreviewAction> {
    match input {
        NoteKeyInput::Token(token) if token == "." || token == "-" => {
            Some(PreviewAction::SilentToken)
        }
        NoteKeyInput::Token(_) => Some(PreviewAction::AuditionPitch),
        NoteKeyInput::Cancel | NoteKeyInput::Unknown => None,
    }
}

fn preview_key_char(key: &KeyEvent) -> Option<char> {
    match normalized_key_stroke(key) {
        Some(KeyStroke::Char(ch))
        | Some(KeyStroke::ShiftChar(ch))
        | Some(KeyStroke::Symbol(ch)) => Some(ch),
        _ => None,
    }
}

enum NoteKeyboardAction {
    OctaveDown,
    OctaveUp,
}

fn note_keyboard_action(keyboard: &NoteKeyboard, key: &KeyEvent) -> Option<NoteKeyboardAction> {
    match normalized_key_stroke(key) {
        Some(KeyStroke::Char(ch)) | Some(KeyStroke::ShiftChar(ch)) => {
            if keyboard.is_octave_down(ch) {
                Some(NoteKeyboardAction::OctaveDown)
            } else if keyboard.is_octave_up(ch) {
                Some(NoteKeyboardAction::OctaveUp)
            } else {
                None
            }
        }
        _ => None,
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
    use super::{
        note_key_input_for_key, parse_note_binding, preview_action, preview_key_char, NoteKeyInput,
        NoteKeyboard, PreviewAction, MAX_KEYBOARD_OCTAVE,
    };
    use crate::config::NoteKeyboardConfig;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
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
    fn keyboard_note_does_not_treat_space_as_note_input() {
        let keyboard = NoteKeyboard::default();
        let input = note_key_input_for_key(
            &keyboard,
            4,
            KeyEvent {
                code: KeyCode::Char(' '),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::empty(),
            },
        );

        assert!(matches!(input, NoteKeyInput::Unknown));
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

    #[test]
    fn preview_action_marks_pitch_tokens_as_audition() {
        assert_eq!(
            preview_action(NoteKeyInput::Token("C4".to_string())),
            Some(PreviewAction::AuditionPitch)
        );
    }

    #[test]
    fn preview_action_marks_rest_and_sustain_as_silent() {
        assert_eq!(
            preview_action(NoteKeyInput::Token(".".to_string())),
            Some(PreviewAction::SilentToken)
        );
        assert_eq!(
            preview_action(NoteKeyInput::Token("-".to_string())),
            Some(PreviewAction::SilentToken)
        );
    }

    #[test]
    fn preview_action_ignores_cancel_and_unknown() {
        assert_eq!(preview_action(NoteKeyInput::Cancel), None);
        assert_eq!(preview_action(NoteKeyInput::Unknown), None);
    }

    #[test]
    fn preview_key_char_normalizes_case() {
        let key = KeyEvent {
            code: KeyCode::Char('A'),
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        assert_eq!(preview_key_char(&key), Some('a'));
    }

    #[test]
    fn preview_key_char_ignores_non_char_keys() {
        let key = KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::empty(),
        };

        assert_eq!(preview_key_char(&key), None);
    }
}
