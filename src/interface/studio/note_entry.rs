use super::input::NOTE_HELP;
use super::StudioApp;
use crossterm::event::{KeyCode, KeyEvent};
use miette::Result;

const MIN_KEYBOARD_OCTAVE: i32 = 0;
const MAX_KEYBOARD_OCTAVE: i32 = 7;

impl StudioApp {
    pub(super) fn handle_note_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.status_message = "Note entry cancelled".into();
            }
            KeyCode::Char('.') => {
                self.place_token_at_current_slot(".")?;
            }
            KeyCode::Char('-') => {
                self.place_token_at_current_slot("-")?;
            }
            KeyCode::Char(ch) => {
                let Some(token) = keyboard_note_token(ch, self.note_keyboard_octave) else {
                    self.status_message = format!("Unknown note key. {}", NOTE_HELP);
                    return Ok(());
                };
                self.place_token_at_current_slot(&token)?;
            }
            _ => {
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
}

fn keyboard_note_token(key: char, base_octave: i32) -> Option<String> {
    let (name, octave_offset) = match key.to_ascii_lowercase() {
        'a' => ("C", 0),
        'w' => ("C#", 0),
        's' => ("D", 0),
        'e' => ("D#", 0),
        'd' => ("E", 0),
        'f' => ("F", 0),
        't' => ("F#", 0),
        'g' => ("G", 0),
        'y' => ("G#", 0),
        'h' => ("A", 0),
        'u' => ("A#", 0),
        'j' => ("B", 0),
        'k' => ("C", 1),
        'o' => ("C#", 1),
        'l' => ("D", 1),
        _ => return None,
    };
    Some(format!("{}{}", name, base_octave + octave_offset))
}

#[cfg(test)]
mod tests {
    use super::keyboard_note_token;

    #[test]
    fn keyboard_note_maps_white_and_black_keys() {
        assert_eq!(keyboard_note_token('a', 4).as_deref(), Some("C4"));
        assert_eq!(keyboard_note_token('w', 4).as_deref(), Some("C#4"));
        assert_eq!(keyboard_note_token('s', 4).as_deref(), Some("D4"));
        assert_eq!(keyboard_note_token('e', 4).as_deref(), Some("D#4"));
        assert_eq!(keyboard_note_token('d', 4).as_deref(), Some("E4"));
        assert_eq!(keyboard_note_token('k', 4).as_deref(), Some("C5"));
        assert_eq!(keyboard_note_token('o', 4).as_deref(), Some("C#5"));
        assert_eq!(keyboard_note_token('l', 4).as_deref(), Some("D5"));
    }

    #[test]
    fn keyboard_note_accepts_uppercase_keys() {
        assert_eq!(keyboard_note_token('A', 3).as_deref(), Some("C3"));
    }
}
