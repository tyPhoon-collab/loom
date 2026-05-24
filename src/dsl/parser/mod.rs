pub mod nom_parsers;
pub mod song_builder;

pub use nom_parsers::{parse_key, parse_line_entry, parse_track_init_command, ParsedLine};

use crate::dsl::error::ParseError;
use crate::dsl::token::{Frontmatter, Song};
use miette::Result;
use nom_parsers::{parse_frontmatter, validate_swing_config};
use song_builder::SongBuilder;

pub fn parse_song(source: String) -> Result<Song, ParseError> {
    let input = source.as_str();

    // Frontmatter
    let (input, metadata) = if input.starts_with("---") {
        match parse_frontmatter(input) {
            Ok(res) => res,
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                return Err(ParseError::from_yaml(
                    e.input,
                    &source,
                    "Invalid Frontmatter YAML".to_string(),
                ));
            }
            Err(_) => panic!("Incomplete input"),
        }
    } else {
        (input, Frontmatter::default())
    };

    if metadata.bpm == 0 || metadata.bpm > 999 {
        return Err(ParseError::from_validation(
            &source[..3], // Point to the start of frontmatter
            &source,
            format!("Invalid BPM: {}", metadata.bpm),
            Some("BPM must be between 1 and 999. Example: bpm: 120".to_string()),
        ));
    }

    let frontmatter_line = source.lines().next().unwrap_or(&source);
    if let Err(msg) = crate::validation::parse_signature(&metadata.signature) {
        return Err(ParseError::from_validation(
            frontmatter_line,
            &source,
            msg,
            Some("Example: signature: 4/4".to_string()),
        ));
    }
    if let Err(msg) = crate::validation::validate_unit(&metadata.unit) {
        return Err(ParseError::from_validation(
            frontmatter_line,
            &source,
            msg,
            Some("Example: unit: bar".to_string()),
        ));
    }
    if let Err(msg) = validate_swing_config(&metadata.swing) {
        return Err(ParseError::from_validation(
            frontmatter_line,
            &source,
            msg,
            Some("Examples: swing: 8, swing: 16, swing: { grid: 8, amount: 66 }".to_string()),
        ));
    }
    if let Some(humanize) = metadata.humanize.values() {
        if let Err(msg) = crate::validation::validate_humanize(humanize.timing, humanize.velocity) {
            return Err(ParseError::from_validation(
                frontmatter_line,
                &source,
                msg,
                Some(
                    "Examples: humanize: true, humanize: { timing: 0.015, velocity: 5, seed: 42 }"
                        .to_string(),
                ),
            ));
        }
    }
    if let Some(loop_range) = &metadata.loop_range {
        if let Err(msg) = crate::validation::parse_loop_range_units(loop_range) {
            return Err(ParseError::from_validation(
                frontmatter_line,
                &source,
                msg,
                Some("Example: loop_range: 0..4".to_string()),
            ));
        }
        if let Err(msg) = crate::validation::beats_per_unit(&metadata.unit, &metadata.signature) {
            return Err(ParseError::from_validation(
                frontmatter_line,
                &source,
                msg,
                Some("Ensure both `unit` and `signature` are valid.".to_string()),
            ));
        }
    }

    let mut builder = SongBuilder::new(&source);

    // Line by line parsing
    for line_str in input.lines() {
        let trimmed = line_str.trim();

        match parse_line_entry(trimmed) {
            Ok((_, parsed)) => match parsed {
                ParsedLine::TrackHeader {
                    name,
                    channel,
                    solo,
                    muted,
                } => {
                    builder.add_track(name, channel, line_str, solo, muted)?;
                }
                ParsedLine::TrackWrap => {
                    builder.add_section(line_str)?;
                }
                ParsedLine::TrackInit { event, .. } => {
                    builder.add_track_init(line_str, event)?;
                }
                ParsedLine::TemplateHeader { name } => {
                    builder.start_template(name);
                }
                ParsedLine::TemplateCalls(calls) => {
                    builder.add_template_calls(calls);
                }
                ParsedLine::Pattern {
                    notes,
                    blocks,
                    end_bar,
                    ..
                } => {
                    builder.add_pattern(line_str, notes, blocks, end_bar)?;
                }
                ParsedLine::Modifier {
                    kind,
                    blocks,
                    end_bar,
                    trailing_comment,
                } => {
                    builder.add_modifier(line_str, kind, blocks, end_bar, trailing_comment)?;
                }
                ParsedLine::Comment(_) | ParsedLine::Empty | ParsedLine::Frontmatter(_) => {}
            },
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                if let Some(rest) = trimmed.strip_prefix("##") {
                    if let Err(msg) = parse_track_init_command(rest.trim()) {
                        return Err(ParseError::from_validation(line_str, &source, msg, None));
                    }
                }
                return Err(ParseError::from_nom(
                    line_str,
                    &source,
                    format!("{:?}", e.code),
                ));
            }
            _ => {}
        }
    }

    let (tracks, templates) = builder.finish();

    Ok(Song {
        metadata,
        tracks,
        templates,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_line_entry, parse_song, ParsedLine};

    #[test]
    fn parse_track_header_accepts_solo_and_mute_flags() {
        let (_, parsed) = parse_line_entry("# Piano: 1 s x").unwrap();
        assert!(matches!(
            parsed,
            ParsedLine::TrackHeader {
                name,
                channel: 1,
                solo: true,
                muted: true,
            } if name == "Piano"
        ));

        let (_, parsed) = parse_line_entry("# Piano: 1 x s").unwrap();
        assert!(matches!(
            parsed,
            ParsedLine::TrackHeader {
                solo: true,
                muted: true,
                ..
            }
        ));
    }

    #[test]
    fn parse_track_header_rejects_unknown_flags() {
        assert!(parse_line_entry("# Piano: 1 z").is_err());
    }

    #[test]
    fn parse_song_tracks_preserve_solo_state() {
        let song = parse_song("# Piano: 1 s\nC4 | ^ |\n".to_string()).unwrap();
        assert_eq!(song.tracks.len(), 1);
        assert!(song.tracks[0].solo);
        assert!(!song.tracks[0].muted);
    }
}
