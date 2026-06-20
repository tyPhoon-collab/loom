pub mod nom_parsers;
pub mod song_builder;

pub use nom_parsers::{parse_key, parse_line_entry, parse_track_init_command, ParsedLine};

use crate::dsl::error::ParseError;
use crate::dsl::token::{FragmentBlock, Frontmatter, Song, TemplateLibrary, Track};
use miette::{IntoDiagnostic, Result};
use nom_parsers::{parse_frontmatter, validate_swing_config};
use song_builder::SongBuilder;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub fn parse_song(source: String) -> Result<Song, ParseError> {
    parse_song_internal(source, None)
}

pub fn parse_song_from_path(path: &Path) -> Result<Song> {
    let source = std::fs::read_to_string(path).into_diagnostic()?;
    let base_dir = path.parent().map(Path::to_path_buf);
    parse_song_internal_with_fragment_overrides(source, base_dir.as_deref(), &HashMap::new())
        .map_err(Into::into)
}

pub fn parse_song_with_base_dir(source: String, base_dir: &Path) -> Result<Song, ParseError> {
    parse_song_internal_with_fragment_overrides(source, Some(base_dir), &HashMap::new())
}

pub fn parse_song_from_path_with_fragment_overrides(
    path: &Path,
    overrides: &HashMap<PathBuf, String>,
) -> Result<Song> {
    let source = std::fs::read_to_string(path).into_diagnostic()?;
    let base_dir = path.parent().map(Path::to_path_buf);
    parse_song_internal_with_fragment_overrides(source, base_dir.as_deref(), overrides)
        .map_err(Into::into)
}

fn parse_song_internal(source: String, base_dir: Option<&Path>) -> Result<Song, ParseError> {
    parse_song_internal_with_fragment_overrides(source, base_dir, &HashMap::new())
}

fn parse_song_internal_with_fragment_overrides(
    source: String,
    base_dir: Option<&Path>,
    fragment_overrides: &HashMap<PathBuf, String>,
) -> Result<Song, ParseError> {
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

    let libraries = load_template_libraries(&metadata.templates, base_dir, &source)?;

    let manifest_calls = collect_fragment_calls(input, &source)?;

    if manifest_calls.is_empty() {
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
                    ParsedLine::FragmentCall { .. } => {
                        return Err(ParseError::from_context(
                            line_str,
                            &source,
                            "Fragment call requires manifest frontmatter mapping".to_string(),
                        ));
                    }
                    ParsedLine::TrackReference { .. } => {
                        return Err(ParseError::from_context(
                            line_str,
                            &source,
                            "Track reference is only allowed inside a song fragment".to_string(),
                        ));
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
            libraries,
            fragment_blocks: Vec::new(),
        })
    } else {
        parse_manifest(
            &source,
            input,
            metadata,
            libraries,
            manifest_calls,
            base_dir,
            fragment_overrides,
        )
    }
}

fn collect_fragment_calls(input: &str, source: &str) -> Result<Vec<(String, String)>, ParseError> {
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

fn parse_manifest(
    source: &str,
    input: &str,
    metadata: Frontmatter,
    libraries: HashMap<String, TemplateLibrary>,
    manifest_calls: Vec<(String, String)>,
    base_dir: Option<&Path>,
    fragment_overrides: &HashMap<PathBuf, String>,
) -> Result<Song, ParseError> {
    let mut builder = SongBuilder::new(source);

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

fn resolve_fragment_path(base_dir: &Path, mapped: &str) -> std::result::Result<PathBuf, String> {
    resolve_mapped_path(base_dir, mapped, "Fragment")
}

fn resolve_template_library_path(
    base_dir: &Path,
    mapped: &str,
) -> std::result::Result<PathBuf, String> {
    resolve_mapped_path(base_dir, mapped, "Template library")
}

fn resolve_mapped_path(
    base_dir: &Path,
    mapped: &str,
    label: &str,
) -> std::result::Result<PathBuf, String> {
    let mapped_path = Path::new(mapped);
    if mapped_path.is_absolute() {
        return Err(format!("{} paths must be relative", label));
    }
    if mapped_path
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(format!("{} paths must not contain parent traversal", label));
    }
    Ok(base_dir.join(mapped_path))
}

fn load_template_libraries(
    mappings: &HashMap<String, String>,
    base_dir: Option<&Path>,
    source: &str,
) -> Result<HashMap<String, TemplateLibrary>, ParseError> {
    if mappings.is_empty() {
        return Ok(HashMap::new());
    }
    let base_dir = base_dir.ok_or_else(|| {
        ParseError::from_context(
            source.lines().next().unwrap_or(source),
            source,
            "Template libraries require parsing from a file path".to_string(),
        )
    })?;
    let mut stack = Vec::new();
    load_template_libraries_from_base(mappings, base_dir, source, &mut stack)
}

fn load_template_libraries_from_base(
    mappings: &HashMap<String, String>,
    base_dir: &Path,
    source: &str,
    stack: &mut Vec<PathBuf>,
) -> Result<HashMap<String, TemplateLibrary>, ParseError> {
    let mut out = HashMap::new();
    for (alias, mapped) in mappings {
        validate_template_library_alias(alias, source)?;
        let path = resolve_template_library_path(base_dir, mapped).map_err(|msg| {
            ParseError::from_validation(source.lines().next().unwrap_or(source), source, msg, None)
        })?;
        out.insert(
            alias.clone(),
            load_template_library(&path, mapped, source, stack)?,
        );
    }
    Ok(out)
}

fn load_template_library(
    path: &Path,
    mapped: &str,
    parent_source: &str,
    stack: &mut Vec<PathBuf>,
) -> Result<TemplateLibrary, ParseError> {
    let normalized = normalize_load_path(path);
    if stack.contains(&normalized) {
        let trace = stack
            .iter()
            .chain(std::iter::once(&normalized))
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        return Err(ParseError::from_validation(
            parent_source.lines().next().unwrap_or(parent_source),
            parent_source,
            format!("Circular template library reference detected: {}", trace),
            None,
        ));
    }

    let source = std::fs::read_to_string(path).map_err(|err| {
        ParseError::from_validation(
            parent_source.lines().next().unwrap_or(parent_source),
            parent_source,
            format!("Cannot read template library '{}': {}", mapped, err),
            None,
        )
    })?;

    stack.push(normalized);
    let result = parse_template_library_source(path, &source, stack);
    stack.pop();
    result
}

fn normalize_load_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn validate_template_library_alias(alias: &str, source: &str) -> Result<(), ParseError> {
    let valid = alias
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
        && alias
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if valid {
        return Ok(());
    }
    Err(ParseError::from_validation(
        source.lines().next().unwrap_or(source),
        source,
        format!("Invalid template library alias '{}'", alias),
        Some(
            "Template library aliases must use ASCII letters, digits, `_`, and `-`, starting with an ASCII letter or digit."
                .to_string(),
        ),
    ))
}

fn parse_template_library_source(
    path: &Path,
    source: &str,
    stack: &mut Vec<PathBuf>,
) -> Result<TemplateLibrary, ParseError> {
    let input = source;
    let (body, metadata) = if input.starts_with("---") {
        validate_template_library_frontmatter(input)?;
        match parse_frontmatter(input) {
            Ok(res) => res,
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                return Err(ParseError::from_yaml(
                    e.input,
                    source,
                    "Invalid Frontmatter YAML".to_string(),
                ));
            }
            Err(_) => panic!("Incomplete input"),
        }
    } else {
        (input, Frontmatter::default())
    };

    let base_dir = path.parent().unwrap_or_else(|| Path::new("."));
    let libraries =
        load_template_libraries_from_base(&metadata.templates, base_dir, source, stack)?;

    let mut builder = SongBuilder::new(source);
    for line_str in body.lines() {
        let trimmed = line_str.trim();
        match parse_line_entry(trimmed) {
            Ok((_, parsed)) => match parsed {
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
                ParsedLine::TrackWrap => builder.add_section(line_str)?,
                ParsedLine::Frontmatter(_)
                | ParsedLine::TrackHeader { .. }
                | ParsedLine::TrackReference { .. }
                | ParsedLine::TrackInit { .. }
                | ParsedLine::FragmentCall { .. } => {
                    return Err(ParseError::from_context(
                        line_str,
                        source,
                        "Template library may contain only frontmatter, template definitions, comments, and blank lines".to_string(),
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

    let (_, templates) = builder.finish();
    Ok(TemplateLibrary {
        source: path.display().to_string(),
        templates,
        libraries,
    })
}

fn validate_template_library_frontmatter(input: &str) -> Result<(), ParseError> {
    let yaml = input
        .strip_prefix("---")
        .and_then(|rest| rest.split_once("---").map(|(yaml, _)| yaml))
        .unwrap_or("");
    let value: serde_yaml::Value = serde_yaml::from_str(yaml)
        .map_err(|_| ParseError::from_yaml(input, input, "Invalid Frontmatter YAML".to_string()))?;
    let Some(mapping) = value.as_mapping() else {
        return Ok(());
    };
    for key in mapping.keys().filter_map(|key| key.as_str()) {
        if !matches!(key, "templates" | "title" | "author") {
            return Err(ParseError::from_validation(
                input.lines().next().unwrap_or(input),
                input,
                format!("Template library frontmatter key '{}' is not allowed", key),
                Some(
                    "Template library frontmatter may contain `templates`, `title`, and `author`."
                        .to_string(),
                ),
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
    let mut builder = SongBuilder::new(&source);
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
