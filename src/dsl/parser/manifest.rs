use super::{parse_line_entry, resolve_fragment_path, ParsedLine};
use crate::dsl::error::ParseError;
use crate::dsl::token::{FragmentBlock, Frontmatter, Song, TemplateLibrary, Track};
use miette::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub(super) fn collect_fragment_calls(
    input: &str,
    source: &str,
) -> Result<Vec<(String, String)>, ParseError> {
    let mut calls = Vec::new();
    for line_str in input.lines() {
        let trimmed = line_str.trim();
        match parse_line_entry(trimmed) {
            Ok((_, ParsedLine::FragmentCall { name })) => calls.push((name, line_str.to_string())),
            Ok(_) => {}
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                if trimmed.starts_with("[[") {
                    return Err(ParseError::from_nom(
                        line_str,
                        source,
                        format!("{:?}", e.code),
                    ));
                }
            }
            Err(_) => {}
        }
    }
    Ok(calls)
}

pub(super) fn parse_manifest(
    source: &str,
    input: &str,
    metadata: Frontmatter,
    libraries: HashMap<String, TemplateLibrary>,
    manifest_calls: Vec<(String, String)>,
    base_dir: Option<&Path>,
    fragment_overrides: &HashMap<PathBuf, String>,
) -> Result<Song, ParseError> {
    let mut builder = super::song_builder::SongBuilder::new(source);

    for line_str in input.lines() {
        let trimmed = line_str.trim();
        match parse_line_entry(trimmed) {
            Ok((_, parsed)) => match parsed {
                ParsedLine::TrackHeader {
                    name,
                    channel,
                    solo,
                    muted,
                } => builder.add_track(name, channel, line_str, solo, muted)?,
                ParsedLine::TrackInit { event, .. } => builder.add_track_init(line_str, event)?,
                ParsedLine::FragmentCall { .. }
                | ParsedLine::Comment(_)
                | ParsedLine::Empty
                | ParsedLine::Frontmatter(_) => {}
                ParsedLine::TrackWrap
                | ParsedLine::TemplateHeader { .. }
                | ParsedLine::TemplateCalls(_)
                | ParsedLine::Pattern { .. }
                | ParsedLine::Modifier { .. }
                | ParsedLine::TrackReference { .. } => {
                    return Err(ParseError::from_context(
                        line_str,
                        source,
                        "Manifest with fragments may contain only frontmatter, track headers, track init, comments, blank lines, and fragment calls".to_string(),
                    ));
                }
            },
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                return Err(ParseError::from_nom(
                    line_str,
                    source,
                    format!("{:?}", e.code),
                ));
            }
            _ => {}
        }
    }

    let (tracks, templates) = builder.finish();
    ensure_unique_manifest_channels(&tracks, source)?;

    let mut fragment_blocks = Vec::new();
    for (name, call_line) in manifest_calls {
        let mapped = metadata.fragments.get(&name).ok_or_else(|| {
            ParseError::from_validation(
                &call_line,
                source,
                format!("Missing fragment mapping for '{}'", name),
                Some("Add `fragments:` frontmatter mapping for this fragment call.".to_string()),
            )
        })?;
        let base_dir = base_dir.ok_or_else(|| {
            ParseError::from_context(
                &call_line,
                source,
                "Fragment calls require parsing from a file path".to_string(),
            )
        })?;
        let path = resolve_fragment_path(base_dir, mapped)
            .map_err(|msg| ParseError::from_validation(&call_line, source, msg, None))?;
        let fragment_source = if let Some(source) = fragment_overrides.get(&path) {
            source.clone()
        } else {
            std::fs::read_to_string(&path).map_err(|err| {
                ParseError::from_validation(
                    &call_line,
                    source,
                    format!("Cannot read fragment '{}': {}", mapped, err),
                    None,
                )
            })?
        };
        fragment_blocks.push(parse_fragment_block(
            &name,
            fragment_source,
            &tracks,
            libraries.clone(),
        )?);
    }

    Ok(Song {
        metadata,
        tracks,
        templates,
        libraries,
        fragment_blocks,
    })
}

fn ensure_unique_manifest_channels(tracks: &[Track], source: &str) -> Result<(), ParseError> {
    let mut seen = HashSet::new();
    for track in tracks {
        if !seen.insert(track.channel) {
            let line = source
                .lines()
                .find(|line| line.trim_start().starts_with('#') && line.contains(':'))
                .unwrap_or(source);
            return Err(ParseError::from_validation(
                line,
                source,
                format!("Duplicate manifest track channel {}", track.channel),
                Some("Track channels must be unique when fragments are used.".to_string()),
            ));
        }
    }
    Ok(())
}

fn parse_fragment_block(
    name: &str,
    source: String,
    manifest_tracks: &[Track],
    libraries: HashMap<String, TemplateLibrary>,
) -> Result<FragmentBlock, ParseError> {
    let manifest_by_channel: HashMap<u8, &Track> = manifest_tracks
        .iter()
        .map(|track| (track.channel, track))
        .collect();
    let mut builder = super::song_builder::SongBuilder::new(&source);
    let mut seen_channels = HashSet::new();

    for line_str in source.lines() {
        let trimmed = line_str.trim();
        match parse_line_entry(trimmed) {
            Ok((_, parsed)) => match parsed {
                ParsedLine::TrackReference { channel } => {
                    if let Err(msg) = crate::validation::ensure_channel_1_based(channel) {
                        return Err(ParseError::from_validation(line_str, &source, msg, None));
                    }
                    let manifest_track = manifest_by_channel.get(&channel).ok_or_else(|| {
                        ParseError::from_validation(
                            line_str,
                            &source,
                            format!("Fragment references undefined manifest channel {}", channel),
                            None,
                        )
                    })?;
                    if !seen_channels.insert(channel) {
                        return Err(ParseError::from_validation(
                            line_str,
                            &source,
                            format!("Duplicate track reference for channel {}", channel),
                            Some(
                                "Each track reference may appear at most once per fragment."
                                    .to_string(),
                            ),
                        ));
                    }
                    builder.add_track(
                        manifest_track.name.clone(),
                        channel,
                        line_str,
                        manifest_track.solo,
                        manifest_track.muted,
                    )?;
                }
                ParsedLine::TrackWrap => builder.add_section(line_str)?,
                ParsedLine::TemplateHeader { name } => builder.start_template(name),
                ParsedLine::TemplateCalls(calls) => builder.add_template_calls(calls),
                ParsedLine::Pattern {
                    notes,
                    blocks,
                    end_bar,
                    ..
                } => builder.add_pattern(line_str, notes, blocks, end_bar)?,
                ParsedLine::Modifier {
                    kind,
                    blocks,
                    end_bar,
                    trailing_comment,
                } => builder.add_modifier(line_str, kind, blocks, end_bar, trailing_comment)?,
                ParsedLine::Comment(_) | ParsedLine::Empty => {}
                ParsedLine::Frontmatter(_)
                | ParsedLine::TrackHeader { .. }
                | ParsedLine::TrackInit { .. }
                | ParsedLine::FragmentCall { .. } => {
                    return Err(ParseError::from_context(
                        line_str,
                        &source,
                        "Fragment may contain only track references, patterns, seq lines, modifiers, templates, track wraps, comments, and blank lines".to_string(),
                    ));
                }
            },
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
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
    Ok(FragmentBlock {
        name: name.to_string(),
        tracks,
        templates,
        libraries,
    })
}
