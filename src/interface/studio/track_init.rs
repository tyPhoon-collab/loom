use super::keystroke::{key_stroke_matches, normalized_key_stroke, KeyStroke};
use super::settings::parse_track_header;
use crate::dsl::token::TrackInitLabel;
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TrackInitKeySpec {
    Cancel,
    Pc,
    Bank,
    Cc,
    Pan,
    Volume,
    Expression,
    Mod,
    Sustain,
}

impl TrackInitKeySpec {
    pub(super) fn label(self) -> &'static str {
        match self {
            TrackInitKeySpec::Cancel => "cancel",
            TrackInitKeySpec::Pc => "pc",
            TrackInitKeySpec::Bank => "bank",
            TrackInitKeySpec::Cc => "cc",
            TrackInitKeySpec::Pan => "pan",
            TrackInitKeySpec::Volume => "volume",
            TrackInitKeySpec::Expression => "expression",
            TrackInitKeySpec::Mod => "mod",
            TrackInitKeySpec::Sustain => "sustain",
        }
    }

    pub(super) fn label_kind(self) -> Option<TrackInitLabel> {
        match self {
            TrackInitKeySpec::Cancel => None,
            TrackInitKeySpec::Pc => Some(TrackInitLabel::Pc),
            TrackInitKeySpec::Bank => Some(TrackInitLabel::Bank),
            TrackInitKeySpec::Cc => Some(TrackInitLabel::Cc),
            TrackInitKeySpec::Pan => Some(TrackInitLabel::Pan),
            TrackInitKeySpec::Volume => Some(TrackInitLabel::Volume),
            TrackInitKeySpec::Expression => Some(TrackInitLabel::Expression),
            TrackInitKeySpec::Mod => Some(TrackInitLabel::Mod),
            TrackInitKeySpec::Sustain => Some(TrackInitLabel::Sustain),
        }
    }
}

pub(super) fn parse_track_init_key(key: KeyEvent) -> Option<TrackInitKeySpec> {
    if key_stroke_matches(KeyStroke::Code(KeyCode::Esc), &key) {
        return Some(TrackInitKeySpec::Cancel);
    }

    match normalized_key_stroke(&key) {
        Some(KeyStroke::Char(ch)) | Some(KeyStroke::ShiftChar(ch)) => match ch {
            'p' => Some(TrackInitKeySpec::Pc),
            'b' => Some(TrackInitKeySpec::Bank),
            'c' => Some(TrackInitKeySpec::Cc),
            'n' => Some(TrackInitKeySpec::Pan),
            'v' => Some(TrackInitKeySpec::Volume),
            'e' => Some(TrackInitKeySpec::Expression),
            'm' => Some(TrackInitKeySpec::Mod),
            's' => Some(TrackInitKeySpec::Sustain),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn current_track_header_row(lines: &[String], cursor_row: usize) -> Option<usize> {
    (0..=cursor_row)
        .rev()
        .find(|&row| {
            lines
                .get(row)
                .and_then(|line| parse_track_header(line))
                .is_some()
        })
        .or_else(|| {
            lines
                .iter()
                .position(|line| parse_track_header(line).is_some())
        })
}

pub(super) fn is_track_init_line(line: &str) -> bool {
    line.trim_start().starts_with("## ")
}

#[cfg(test)]
mod tests {
    use super::{current_track_header_row, parse_track_init_key, TrackInitKeySpec};
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        }
    }

    #[test]
    fn current_track_header_row_finds_enclosing_track() {
        let lines = vec![
            "# Piano: 1".to_string(),
            "## pc 4".to_string(),
            "seq | C4 |".to_string(),
            "# Bass: 2".to_string(),
        ];
        assert_eq!(current_track_header_row(&lines, 2), Some(0));
        assert_eq!(current_track_header_row(&lines, 3), Some(3));
    }

    #[test]
    fn parse_track_init_key_maps_supported_bindings() {
        assert_eq!(
            parse_track_init_key(key(KeyCode::Char('p'), KeyModifiers::NONE)),
            Some(TrackInitKeySpec::Pc)
        );
        assert_eq!(
            parse_track_init_key(key(KeyCode::Char('n'), KeyModifiers::NONE)),
            Some(TrackInitKeySpec::Pan)
        );
        assert_eq!(
            parse_track_init_key(key(KeyCode::Char('s'), KeyModifiers::NONE)),
            Some(TrackInitKeySpec::Sustain)
        );
        assert_eq!(
            parse_track_init_key(key(KeyCode::Char('v'), KeyModifiers::NONE)),
            Some(TrackInitKeySpec::Volume)
        );
    }
}
