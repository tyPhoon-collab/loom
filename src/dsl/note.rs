use miette::{miette, Result};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub enum Note {
    Pitch { name: String, octave: i32 },
    Drum(String),
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
        }
    }
}

impl Note {
    pub fn to_midi(&self) -> u8 {
        match self {
            Note::Pitch { name, octave } => {
                let offset = match name.as_str() {
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
                    _ => 0,
                };
                ((octave + 2) * 12 + offset).clamp(0, 127) as u8
            }
            Note::Drum(alias) => match alias.as_str() {
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
                _ => 36, // Fallback - should not happen if parsed correctly
            },
        }
    }
}

impl FromStr for Note {
    type Err = miette::Report;

    fn from_str(s: &str) -> Result<Self> {
        // 1. Check Drum Aliases (Case-Sensitive)
        match s {
            "bd" | "kick" | "sn" | "snare" | "rs" | "rim" | "cp" | "clap" | "hh" | "hc"
            | "hihat" | "oh" | "ho" | "hp" | "cr" | "crash" | "rd" | "ride" | "splash"
            | "china" | "ht" | "mt" | "lt" | "ft" | "cb" | "tamb" => {
                return Ok(Note::Drum(s.to_string()));
            }
            _ => {}
        }

        // 2. Parse Pitch (Case-Insensitive for the pitch name part)
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

        if pitch_part.is_empty() {
            return Err(miette!("Invalid note name: {}", s));
        }

        // Validate pitch_part
        match pitch_part.as_str() {
            "c" | "c#" | "db" | "d" | "d#" | "eb" | "e" | "f" | "f#" | "gb" | "g" | "g#" | "ab"
            | "a" | "a#" | "bb" | "b" => {}
            _ => return Err(miette!("Invalid pitch name: {}", pitch_part)),
        }

        let octave = octave_part.parse::<i32>().unwrap_or(3);
        Ok(Note::Pitch {
            name: pitch_part,
            octave,
        })
    }
}
