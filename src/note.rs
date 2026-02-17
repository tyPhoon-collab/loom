use miette::{miette, Result};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub enum Note {
    Pitch { name: String, octave: i32 },
    Drum(String),
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
                "bd" | "kick" | "bassdrum" => 36,
                // Snare
                "sd" | "snare" => 38,
                "rim" | "rs" | "sidestick" => 37,
                "clap" | "handclap" | "cp" => 39,
                // Hi-hat
                "hc" | "hihat" | "hihatclosed" => 42,
                "ho" | "hihatopen" => 46,
                "hp" | "hihatpedal" => 44,
                // Cymbals
                "crash" => 49,
                "ride" => 51,
                "splash" => 55,
                "china" => 52,
                // Toms
                "ht" | "himidtom" => 48,
                "mt" | "lowmidtom" => 47,
                "lt" | "lowtom" => 45,
                "ft" | "highfloortom" => 43,
                // Others
                "cb" | "cowbell" => 56,
                "tamb" | "tambourine" => 54,
                _ => 36, // Fallback - should not happen if parsed correctly
            },
        }
    }
}

impl FromStr for Note {
    type Err = miette::Report;

    fn from_str(s: &str) -> Result<Self> {
        let s = s.to_lowercase();

        // Check Drum Aliases
        match s.as_str() {
            "bd" | "kick" | "bassdrum" | "sd" | "snare" | "rim" | "rs" | "sidestick" | "clap"
            | "handclap" | "cp" | "hc" | "hihat" | "hihatclosed" | "ho" | "hihatopen" | "hp"
            | "hihatpedal" | "crash" | "ride" | "splash" | "china" | "ht" | "himidtom" | "mt"
            | "lowmidtom" | "lt" | "lowtom" | "ft" | "highfloortom" | "cb" | "cowbell" | "tamb"
            | "tambourine" => {
                return Ok(Note::Drum(s));
            }
            _ => {}
        }

        // Parse Pitch
        let mut pitch_part = String::new();
        let mut octave_part = String::new();
        let mut chars = s.chars().peekable();

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
