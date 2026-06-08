use super::input::{NoteInputMode, PendingInput, NOTE_HELP};
use super::keystroke::{key_stroke_matches, normalized_key_stroke, KeyStroke};
use super::settings::parse_track_header;
use super::StudioApp;
use crate::config::NoteKeyboardConfig;
use crate::dsl::parser::parse_track_init_command;
use crate::dsl::token::TrackInitEvent;
use crate::dsl::Note;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use miette::Result;
use std::collections::HashMap;

const MIN_KEYBOARD_OCTAVE: i32 = 0;
const MAX_KEYBOARD_OCTAVE: i32 = 7;
const DEFAULT_KEYBOARD_OCTAVE: i32 = 4;
const PREVIEW_PROGRAM_STEP: u8 = 1;
const PREVIEW_PROGRAM_PAGE_STEP: u8 = 10;

#[derive(Clone, Debug)]
pub(super) struct NoteKeyboard {
    keys: HashMap<char, NoteKeyBinding>,
    octave_down: char,
    octave_up: char,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct KeyboardVisualKey {
    pub(super) physical_key: char,
    pub(super) note_name: &'static str,
    pub(super) octave_offset: i32,
    pub(super) is_black: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct KeyboardVisualLayout {
    pub(super) pitch_keys: Vec<KeyboardVisualKey>,
    pub(super) rest_key: char,
    pub(super) sustain_key: char,
    pub(super) octave_down_key: char,
    pub(super) octave_up_key: char,
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
            keyboard.set_binding(key, binding);
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

    fn set_binding(&mut self, key: char, binding: NoteKeyBinding) {
        self.keys.retain(|existing_key, existing_binding| {
            *existing_key == key || *existing_binding != binding
        });
        self.keys.insert(key, binding);
    }

    pub(super) fn visual_layout(&self) -> KeyboardVisualLayout {
        KeyboardVisualLayout {
            pitch_keys: VISUAL_PITCH_LAYOUT
                .iter()
                .filter_map(|(note_name, octave_offset, is_black)| {
                    self.find_pitch_key(note_name, *octave_offset)
                        .map(|physical_key| KeyboardVisualKey {
                            physical_key,
                            note_name,
                            octave_offset: *octave_offset,
                            is_black: *is_black,
                        })
                })
                .collect(),
            rest_key: self.find_rest_key().unwrap_or('.'),
            sustain_key: self.find_sustain_key().unwrap_or('-'),
            octave_down_key: self.octave_down,
            octave_up_key: self.octave_up,
        }
    }

    fn find_pitch_key(&self, name: &str, octave_offset: i32) -> Option<char> {
        self.keys.iter().find_map(|(key, binding)| match binding {
            NoteKeyBinding::Pitch {
                name: binding_name,
                octave_offset: binding_offset,
            } if binding_name == name && *binding_offset == octave_offset => Some(*key),
            _ => None,
        })
    }

    fn find_rest_key(&self) -> Option<char> {
        self.keys.iter().find_map(|(key, binding)| match binding {
            NoteKeyBinding::Rest => Some(*key),
            _ => None,
        })
    }

    fn find_sustain_key(&self) -> Option<char> {
        self.keys.iter().find_map(|(key, binding)| match binding {
            NoteKeyBinding::Sustain => Some(*key),
            _ => None,
        })
    }
}

const VISUAL_PITCH_LAYOUT: &[(&str, i32, bool)] = &[
    ("C", 0, false),
    ("C#", 0, true),
    ("D", 0, false),
    ("D#", 0, true),
    ("E", 0, false),
    ("F", 0, false),
    ("F#", 0, true),
    ("G", 0, false),
    ("G#", 0, true),
    ("A", 0, false),
    ("A#", 0, true),
    ("B", 0, false),
    ("C", 1, false),
    ("C#", 1, true),
    ("D", 1, false),
    ("D#", 1, true),
    ("E", 1, false),
];

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
            ('p', "D#", 1),
            (';', "E", 1),
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
    pub(super) fn handle_preview_panel_key(&mut self, key: KeyEvent) -> Result<()> {
        if key.kind == KeyEventKind::Release {
            if let Some(ch) = preview_key_char(&key) {
                self.preview_panel.active_keys.remove(&ch);
                if let Some(active) = self.active_preview_keys.remove(&ch) {
                    self.player.preview_note_off(active.channel, active.note);
                }
            }
            return Ok(());
        }

        if key_stroke_matches(KeyStroke::Code(KeyCode::Esc), &key)
            || key_stroke_matches(KeyStroke::ShiftChar('p'), &key)
        {
            self.close_preview_panel();
            return Ok(());
        }
        if key_stroke_matches(KeyStroke::Code(KeyCode::Enter), &key) {
            self.apply_preview_program()?;
            return Ok(());
        }

        match normalized_key_stroke(&key) {
            Some(KeyStroke::Symbol('[')) => {
                self.adjust_preview_program(-i16::from(PREVIEW_PROGRAM_STEP));
                return Ok(());
            }
            Some(KeyStroke::Symbol(']')) => {
                self.adjust_preview_program(i16::from(PREVIEW_PROGRAM_STEP));
                return Ok(());
            }
            Some(KeyStroke::Symbol('{')) => {
                self.adjust_preview_program(-i16::from(PREVIEW_PROGRAM_PAGE_STEP));
                return Ok(());
            }
            Some(KeyStroke::Symbol('}')) => {
                self.adjust_preview_program(i16::from(PREVIEW_PROGRAM_PAGE_STEP));
                return Ok(());
            }
            Some(KeyStroke::Char('r')) => {
                self.preview_panel.override_program = None;
                self.status_message = match self.preview_panel.source_program {
                    Some(program) => format!("Preview program reset to source pc {}", program),
                    None => "Preview program reset to track default".to_string(),
                };
                return Ok(());
            }
            _ => {}
        }

        match note_keyboard_action(&self.note_keyboard, &key) {
            Some(NoteKeyboardAction::OctaveDown) => {
                self.preview_panel
                    .active_keys
                    .insert(self.note_keyboard.octave_down);
                self.adjust_note_keyboard_octave(-1);
                return Ok(());
            }
            Some(NoteKeyboardAction::OctaveUp) => {
                self.preview_panel
                    .active_keys
                    .insert(self.note_keyboard.octave_up);
                self.adjust_note_keyboard_octave(1);
                return Ok(());
            }
            None => {}
        }

        let Some(ch) = preview_key_char(&key) else {
            match self.note_key_input(key) {
                NoteKeyInput::Cancel => {
                    self.clear_active_preview_notes();
                    self.close_preview_panel();
                }
                NoteKeyInput::Unknown => {
                    self.status_message = "Unknown preview key".into();
                }
                NoteKeyInput::Token(_) => unreachable!(),
            }
            return Ok(());
        };

        self.preview_panel.active_keys.insert(ch);
        if self.active_preview_keys.contains_key(&ch) {
            return Ok(());
        }

        match preview_action(self.note_key_input(key)) {
            Some(PreviewAction::AuditionPitch) => {
                let NoteKeyInput::Token(token) = self.note_key_input(key) else {
                    unreachable!();
                };
                if self.is_playing {
                    self.status_message = format!("Preview suppressed while playing: {}", token);
                } else if let Some((channel, note)) =
                    self.preview_target_for_channel(self.preview_panel.channel, &token)
                {
                    if let Some(program) = self.preview_panel.effective_program() {
                        self.player.preview_program_change(channel, program);
                    }
                    self.player.preview_note_on(channel, note, 96);
                    self.active_preview_keys
                        .insert(ch, super::ActivePreviewNote { channel, note });
                    self.status_message = match self.preview_panel.effective_program() {
                        Some(program) => {
                            format!("Preview: {} | ch {} | pc {}", token, channel + 1, program)
                        }
                        None => format!("Preview: {} | ch {}", token, channel + 1),
                    };
                } else {
                    self.active_preview_keys.remove(&ch);
                    self.status_message = format!("Preview unavailable here: {}", token);
                }
            }
            Some(PreviewAction::SilentToken) => {
                let NoteKeyInput::Token(token) = self.note_key_input(key) else {
                    unreachable!();
                };
                self.active_preview_keys.remove(&ch);
                self.status_message = format!("Preview silent: {}", token);
            }
            None => match self.note_key_input(key) {
                NoteKeyInput::Cancel => self.close_preview_panel(),
                NoteKeyInput::Unknown => {
                    self.status_message = "Unknown preview key".into();
                }
                NoteKeyInput::Token(_) => unreachable!(),
            },
        }
        Ok(())
    }

    pub(super) fn toggle_preview_panel(&mut self) {
        if self.preview_panel.open {
            self.close_preview_panel();
            return;
        }

        let row = self.textarea.cursor().0;
        let Some(context) = self.preview_track_context_for_row(row) else {
            self.status_message = "Preview panel needs the cursor inside a track".into();
            return;
        };

        self.clear_active_preview_notes();
        self.preview_panel.open = true;
        self.preview_panel.track_header_row = Some(context.track_header_row);
        self.preview_panel.track_name = context.track_name;
        self.preview_panel.channel = context.channel;
        self.preview_panel.source_program = context.source_program;
        self.preview_panel.override_program = None;
        self.preview_panel.velocity = 96;
        self.preview_panel.active_keys.clear();
        self.status_message = format!(
            "Preview panel: {} | ch {}{}",
            self.preview_panel.track_name,
            self.preview_panel.channel + 1,
            self.preview_panel
                .source_program
                .map(|program| format!(" | source pc {}", program))
                .unwrap_or_default()
        );
    }

    pub(super) fn close_preview_panel(&mut self) {
        self.clear_active_preview_notes();
        self.preview_panel.open = false;
        self.preview_panel.track_header_row = None;
        self.preview_panel.active_keys.clear();
        self.status_message = "Preview panel closed".into();
    }

    fn adjust_preview_program(&mut self, delta: i16) {
        let current = self.preview_panel.effective_program().unwrap_or(0);
        let next = (i16::from(current) + delta).clamp(0, 127) as u8;
        self.preview_panel.override_program = Some(next);
        self.status_message = format!(
            "Preview program: {} (track {} ch {})",
            next,
            self.preview_panel.track_name,
            self.preview_panel.channel + 1
        );
    }

    fn apply_preview_program(&mut self) -> Result<()> {
        let Some(program) = self.preview_panel.effective_program() else {
            self.status_message = "No preview pc to apply".into();
            return Ok(());
        };
        let Some(track_header_row) = self.preview_panel.track_header_row else {
            self.status_message = "Preview apply needs a track target".into();
            return Ok(());
        };

        let mut lines = self.textarea.lines().to_vec();
        let Some(applied_row) = apply_track_program_to_lines(&mut lines, track_header_row, program)
        else {
            self.status_message = "Preview apply needs a track target".into();
            return Ok(());
        };
        let track_name = self.preview_panel.track_name.clone();
        self.apply_cursor_source_update(
            lines,
            (applied_row, 0),
            format!("Applied pc {} to {}", program, track_name),
            None,
        )?;
        self.preview_panel.source_program = Some(program);
        self.preview_panel.override_program = None;
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

impl StudioApp {
    fn preview_target_for_channel(&self, channel: u8, token: &str) -> Option<(u8, u8)> {
        let note = token.parse::<Note>().ok()?;
        let midi = note.to_midi_checked().ok()?;
        let channel = match note {
            Note::Drum(_) => 9,
            _ => channel,
        };
        Some((channel, midi))
    }
}

fn apply_track_program_to_lines(
    lines: &mut Vec<String>,
    track_header_row: usize,
    program: u8,
) -> Option<usize> {
    parse_track_header(lines.get(track_header_row)?)?;
    let replacement = format!("## pc {}", program);
    let track_end = lines
        .iter()
        .enumerate()
        .skip(track_header_row + 1)
        .find_map(|(row, line)| {
            (parse_track_header(line).is_some() || line.trim().starts_with("# @")).then_some(row)
        })
        .unwrap_or(lines.len());

    if let Some(row) =
        (track_header_row + 1..track_end).find(|&row| is_program_init_line(&lines[row]))
    {
        lines[row] = replacement;
        return Some(row);
    }

    let insert_row = track_header_row + 1;
    lines.insert(insert_row, replacement);
    Some(insert_row)
}

fn is_program_init_line(line: &str) -> bool {
    let Some(command) = line.trim().strip_prefix("## ") else {
        return false;
    };
    matches!(
        parse_track_init_command(command),
        Ok((TrackInitEvent::ProgramChange { .. }, _))
    )
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
        apply_track_program_to_lines, note_key_input_for_key, parse_note_binding, preview_action,
        preview_key_char, NoteKeyInput, NoteKeyboard, PreviewAction, MAX_KEYBOARD_OCTAVE,
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
        assert_eq!(keyboard.token('p', 4).as_deref(), Some("D#5"));
        assert_eq!(keyboard.token(';', 4).as_deref(), Some("E5"));
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
    fn visual_layout_prefers_configured_override_over_default_binding() {
        let mut keys = HashMap::new();
        keys.insert("a".to_string(), "E".to_string());
        let config = NoteKeyboardConfig {
            base_octave: Some(4),
            octave_down: None,
            octave_up: None,
            keys,
        };

        let (keyboard, _) = NoteKeyboard::from_config(&config);
        let layout = keyboard.visual_layout();
        let e_key = layout
            .pitch_keys
            .iter()
            .find(|key| key.note_name == "E" && key.octave_offset == 0)
            .map(|key| key.physical_key);

        assert_eq!(e_key, Some('a'));
        assert_eq!(keyboard.token('d', 4), None);
        assert_eq!(keyboard.token('a', 4).as_deref(), Some("E4"));
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

    #[test]
    fn visual_layout_covers_default_c_through_high_e() {
        let layout = NoteKeyboard::default().visual_layout();

        let keys: Vec<char> = layout
            .pitch_keys
            .iter()
            .map(|key| key.physical_key)
            .collect();
        assert_eq!(
            keys,
            vec![
                'a', 'w', 's', 'e', 'd', 'f', 't', 'g', 'y', 'h', 'u', 'j', 'k', 'o', 'l', 'p', ';'
            ]
        );
        assert_eq!(layout.rest_key, '.');
        assert_eq!(layout.sustain_key, '-');
        assert_eq!(layout.octave_down_key, 'z');
        assert_eq!(layout.octave_up_key, 'x');
    }

    #[test]
    fn apply_track_program_replaces_existing_pc() {
        let mut lines = vec![
            "# Lead: 1".to_string(),
            "## pc 12".to_string(),
            "C4 | ^ |".to_string(),
        ];

        assert_eq!(apply_track_program_to_lines(&mut lines, 0, 42), Some(1));
        assert_eq!(
            lines,
            vec![
                "# Lead: 1".to_string(),
                "## pc 42".to_string(),
                "C4 | ^ |".to_string(),
            ]
        );
    }

    #[test]
    fn apply_track_program_normalizes_sound_alias() {
        let mut lines = vec![
            "# Lead: 1".to_string(),
            "## sound 81".to_string(),
            "C4 | ^ |".to_string(),
        ];

        assert_eq!(apply_track_program_to_lines(&mut lines, 0, 7), Some(1));
        assert_eq!(lines[1], "## pc 7");
    }

    #[test]
    fn apply_track_program_inserts_below_header_when_missing() {
        let mut lines = vec!["# Lead: 1".to_string(), "C4 | ^ |".to_string()];

        assert_eq!(apply_track_program_to_lines(&mut lines, 0, 33), Some(1));
        assert_eq!(
            lines,
            vec![
                "# Lead: 1".to_string(),
                "## pc 33".to_string(),
                "C4 | ^ |".to_string(),
            ]
        );
    }

    #[test]
    fn apply_track_program_does_not_cross_track_boundary() {
        let mut lines = vec![
            "# Lead: 1".to_string(),
            "C4 | ^ |".to_string(),
            "# Bass: 2".to_string(),
            "## pc 34".to_string(),
        ];

        assert_eq!(apply_track_program_to_lines(&mut lines, 0, 99), Some(1));
        assert_eq!(lines[1], "## pc 99");
        assert_eq!(lines[4], "## pc 34");
    }
}
