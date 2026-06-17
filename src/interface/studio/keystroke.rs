use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum KeyStroke {
    Char(char),
    Symbol(char),
    ShiftChar(char),
    CtrlChar(char),
    Code(KeyCode),
    ShiftCode(KeyCode),
}

#[derive(Clone, Copy, Debug)]
pub(super) struct KeyBinding<T> {
    pub(super) stroke: KeyStroke,
    pub(super) action: T,
}

pub(super) fn lookup_key_action<T: Copy>(bindings: &[KeyBinding<T>], key: &KeyEvent) -> Option<T> {
    bindings
        .iter()
        .find(|binding| key_stroke_matches(binding.stroke, key))
        .map(|binding| binding.action)
}

pub(super) fn key_stroke_matches(stroke: KeyStroke, key: &KeyEvent) -> bool {
    normalized_key_stroke(key).is_some_and(|normalized| normalized == stroke)
}

pub(super) fn normalized_key_stroke(key: &KeyEvent) -> Option<KeyStroke> {
    match key.code {
        KeyCode::Char(code) if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(KeyStroke::CtrlChar(code.to_ascii_lowercase()))
        }
        KeyCode::Char(code) => normalize_symbol(code, key.modifiers)
            .map(KeyStroke::Symbol)
            .or_else(|| {
                if code.is_ascii_uppercase() || key.modifiers.contains(KeyModifiers::SHIFT) {
                    Some(KeyStroke::ShiftChar(code.to_ascii_lowercase()))
                } else {
                    Some(KeyStroke::Char(code.to_ascii_lowercase()))
                }
            }),
        code if key.modifiers.contains(KeyModifiers::SHIFT) => Some(KeyStroke::ShiftCode(code)),
        code => Some(KeyStroke::Code(code)),
    }
}

fn normalize_symbol(code: char, modifiers: KeyModifiers) -> Option<char> {
    let shift = modifiers.contains(KeyModifiers::SHIFT);
    match (code, shift) {
        ('/', true) | ('?', _) => Some('?'),
        (';', true) | (':', _) => Some(':'),
        (',', true) | ('<', _) => Some('<'),
        ('.', true) | ('>', _) => Some('>'),
        ('[', true) | ('{', _) => Some('{'),
        (']', true) | ('}', _) => Some('}'),
        ('=', true) | ('+', _) => Some('+'),
        ('.', false) | ('-', false) | (',', false) | ('[', false) | (']', false) | ('=', false) => {
            Some(code)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{key_stroke_matches, normalized_key_stroke, KeyStroke};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn help_key_accepts_shift_slash() {
        let key = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::SHIFT);
        assert!(key_stroke_matches(KeyStroke::Symbol('?'), &key));
    }

    #[test]
    fn shifted_comma_normalizes_to_left_angle() {
        let key = KeyEvent::new(KeyCode::Char(','), KeyModifiers::SHIFT);
        assert!(key_stroke_matches(KeyStroke::Symbol('<'), &key));
    }

    #[test]
    fn shifted_semicolon_normalizes_to_colon() {
        let key = KeyEvent::new(KeyCode::Char(';'), KeyModifiers::SHIFT);
        assert!(key_stroke_matches(KeyStroke::Symbol(':'), &key));
    }

    #[test]
    fn plain_symbol_stays_plain() {
        let key = KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE);
        assert_eq!(normalized_key_stroke(&key), Some(KeyStroke::Symbol('.')));
    }

    #[test]
    fn uppercase_char_without_shift_normalizes_to_shift_char() {
        let key = KeyEvent::new(KeyCode::Char('P'), KeyModifiers::NONE);
        assert_eq!(normalized_key_stroke(&key), Some(KeyStroke::ShiftChar('p')));
    }
}
