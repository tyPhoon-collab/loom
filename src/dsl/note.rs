use miette::{miette, Result};
use phf::phf_map;
use std::str::FromStr;

/// ピッチ名 → 半音オフセット (C=0, C#/Db=1, ..., B=11)
static PITCH_MAP: phf::Map<&str, u8> = phf_map! {
    "c" => 0,
    "c#" | "db" => 1,
    "d" => 2,
    "d#" | "eb" => 3,
    "e" => 4,
    "f" => 5,
    "f#" | "gb" => 6,
    "g" => 7,
    "g#" | "ab" => 8,
    "a" => 9,
    "a#" | "bb" => 10,
    "b" => 11,
};

/// ドラムエイリアス → MIDIノート番号 (GM Percussion Map)
static DRUM_MAP: phf::Map<&str, u8> = phf_map! {
    // Kick
    "bd" | "kick" => 36,
    // Snare
    "sn" | "snare" => 38,
    "rs" | "rim" => 37,
    "cp" | "clap" => 39,
    // Hi-hat
    "hh" | "hc" | "hihat" => 42,
    "oh" | "ho" => 46,
    "hp" => 44,
    // Cymbals
    "cr" | "crash" => 49,
    "rd" | "ride" => 51,
    "splash" => 55,
    "china" => 52,
    // Toms
    "ht" => 48,
    "mt" => 47,
    "lt" => 45,
    "ft" => 43,
    // Others
    "cb" => 56,
    "tamb" => 54,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Note {
    Pitch { name: String, octave: i32 },
    Drum(String),
    Midi(u8),
}

impl std::fmt::Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Note::Pitch { name, octave } => {
                let mut chars = name.chars();
                if let Some(first) = chars.next() {
                    let first_upper = first.to_uppercase().to_string();
                    let rest = chars.collect::<String>();
                    write!(f, "{}{}{}", first_upper, rest, octave)
                } else {
                    write!(f, "{}", octave)
                }
            }
            Note::Drum(alias) => write!(f, "{}", alias),
            Note::Midi(v) => write!(f, "{}", v),
        }
    }
}

impl Note {
    pub fn to_midi_checked(&self) -> Result<u8> {
        match self {
            Note::Pitch { name, octave } => {
                let offset = PITCH_MAP
                    .get(name.as_str())
                    .copied()
                    .ok_or_else(|| miette!("Invalid pitch name: {}", name))?;
                let midi = (octave + 2) * 12 + offset as i32;
                crate::validation::ensure_u7_i32(midi, "MIDI note")
                    .map_err(|e| miette!("{} ({})", e, self))
            }
            Note::Drum(alias) => DRUM_MAP
                .get(alias.as_str())
                .copied()
                .ok_or_else(|| miette!("Invalid drum alias: {}", alias)),
            Note::Midi(v) => Ok(*v),
        }
    }

    pub fn to_midi(&self) -> u8 {
        self.to_midi_checked()
            .expect("validated note must be convertible to MIDI")
    }
}

impl FromStr for Note {
    type Err = miette::Report;

    fn from_str(s: &str) -> Result<Self> {
        // 1. Check if it's a numeric MIDI note (always valid up to 127)
        if let Ok(midi_val) = s.parse::<u8>() {
            if midi_val <= 127 {
                return Ok(Note::Midi(midi_val));
            }
        }

        // 2. Check Drum Aliases (Case-Sensitive)
        if DRUM_MAP.contains_key(s) {
            return Ok(Note::Drum(s.to_string()));
        }

        // 3. Parse Pitch (Case-Insensitive for the pitch name part)
        let s_lower = s.to_lowercase();
        let mut pitch_part = String::new();
        let mut octave_part = String::new();
        let mut chars = s_lower.chars().peekable();

        while let Some(&c) = chars.peek() {
            if c.is_alphabetic() || c == '#' || c == 'b' {
                pitch_part.push(chars.next().unwrap());
            } else {
                break;
            }
        }
        while let Some(&c) = chars.peek() {
            if c.is_ascii_digit() || (c == '-' && octave_part.is_empty()) {
                octave_part.push(chars.next().unwrap());
            } else {
                break;
            }
        }

        if chars.peek().is_some() {
            return Err(miette!("Invalid note format: {}", s));
        }

        if pitch_part.is_empty() {
            return Err(miette!("Invalid note name: {}", s));
        }

        // Validate pitch_part
        if !PITCH_MAP.contains_key(pitch_part.as_str()) {
            return Err(miette!("Invalid pitch name: {}", pitch_part));
        }

        let octave = if octave_part.is_empty() {
            3
        } else {
            octave_part
                .parse::<i32>()
                .map_err(|_| miette!("Invalid octave: {}", octave_part))?
        };
        Ok(Note::Pitch {
            name: pitch_part,
            octave,
        })
    }
}
