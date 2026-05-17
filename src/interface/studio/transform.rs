use super::selection::is_note_token_char;
use crate::dsl::note::Note;
use miette::Result;

pub(super) fn transpose_line(line: &str, semitones: i32) -> Result<(String, bool)> {
    let Some(pipe_idx) = line.find('|') else {
        return Ok((line.to_string(), false));
    };
    let (head, tail) = line.split_at(pipe_idx);
    if head.trim() == "seq" {
        let (new_tail, changed) = transpose_note_tokens(tail, semitones)?;
        if !changed {
            return Ok((line.to_string(), false));
        }
        return Ok((format!("{}{}", head, new_tail), true));
    }

    let (new_head, changed) = transpose_note_head(head, semitones)?;
    if !changed {
        return Ok((line.to_string(), false));
    }
    Ok((format!("{}{}", new_head, tail), true))
}

pub(super) fn transpose_bar_text(input: &str, semitones: i32) -> Result<(String, bool)> {
    transpose_note_tokens(input, semitones)
}

fn transpose_note_head(head: &str, semitones: i32) -> Result<(String, bool)> {
    transpose_note_tokens(head, semitones)
}

fn transpose_note_tokens(input: &str, semitones: i32) -> Result<(String, bool)> {
    let mut out = String::with_capacity(input.len());
    let mut token = String::new();
    let mut changed = false;

    for ch in input.chars() {
        if is_note_token_char(ch) {
            token.push(ch);
        } else {
            if !token.is_empty() {
                out.push_str(&transpose_note_token(&token, semitones, &mut changed)?);
                token.clear();
            }
            out.push(ch);
        }
    }

    if !token.is_empty() {
        out.push_str(&transpose_note_token(&token, semitones, &mut changed)?);
    }

    Ok((out, changed))
}

pub(super) fn transpose_note_token(
    token: &str,
    semitones: i32,
    changed: &mut bool,
) -> Result<String> {
    let note = match token.parse::<Note>() {
        Ok(note) => note,
        Err(_) => return Ok(token.to_string()),
    };

    if matches!(note, Note::Drum(_)) {
        return Ok(token.to_string());
    }

    let midi = i32::from(note.to_midi_checked()?) + semitones;
    if !(0..=127).contains(&midi) {
        return Err(miette::miette!(
            "Transpose result out of MIDI range for {}: {}",
            token,
            midi
        ));
    }

    *changed = true;
    Ok(midi_to_loom_pitch(midi as u8))
}

fn midi_to_loom_pitch(midi: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = i32::from(midi / 12) - 2;
    let name = NAMES[usize::from(midi % 12)];
    format!("{}{}", name, octave)
}

#[cfg(test)]
mod tests {
    use super::{transpose_bar_text, transpose_line};

    #[test]
    fn transpose_pitch_list_before_bar() {
        let (line, changed) = transpose_line("F4,C5 | ^ . |", 2).unwrap();
        assert!(changed);
        assert_eq!(line, "G4,D5 | ^ . |");
    }

    #[test]
    fn transpose_keeps_drums() {
        let (line, changed) = transpose_line("kick | ^ . |", 2).unwrap();
        assert!(!changed);
        assert_eq!(line, "kick | ^ . |");
    }

    #[test]
    fn transpose_numeric_midi_to_pitch() {
        let (line, changed) = transpose_line("60 | ^ . |", 1).unwrap();
        assert!(changed);
        assert_eq!(line, "C#3 | ^ . |");
    }

    #[test]
    fn transpose_seq_body_notes() {
        let (line, changed) = transpose_line("seq | D4 . Eb4 A#3 |", 1).unwrap();
        assert!(changed);
        assert_eq!(line, "seq | D#4 . E4 B3 |");
    }

    #[test]
    fn transpose_bar_text_transposes_only_notes() {
        let (bar, changed) = transpose_bar_text("| D4 . - Eb4 |", 1).unwrap();
        assert!(changed);
        assert_eq!(bar, "| D#4 . - E4 |");
    }
}
