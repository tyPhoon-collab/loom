pub mod nom_parsers;
pub mod song_builder;

mod manifest;
mod template_library;

pub use nom_parsers::{parse_key, parse_line_entry, parse_track_init_command, ParsedLine};

use crate::dsl::error::ParseError;
use crate::dsl::token::{Frontmatter, Song};
use miette::{IntoDiagnostic, Result};
use nom_parsers::{parse_frontmatter, validate_swing_config};
use song_builder::SongBuilder;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

pub(super) trait SourceResolver {
    fn read_to_string(&self, path: &Path) -> std::result::Result<String, String>;

    fn normalize_load_path(&self, path: &Path) -> PathBuf {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    }
}

struct FsSourceResolver;

impl SourceResolver for FsSourceResolver {
    fn read_to_string(&self, path: &Path) -> std::result::Result<String, String> {
        std::fs::read_to_string(path).map_err(|err| err.to_string())
    }
}

struct VirtualSourceResolver {
    files: HashMap<PathBuf, String>,
}

impl SourceResolver for VirtualSourceResolver {
    fn read_to_string(&self, path: &Path) -> std::result::Result<String, String> {
        let normalized = normalize_virtual_path(path).map_err(|err| err.to_string())?;
        self.files
            .get(&normalized)
            .cloned()
            .ok_or_else(|| "file not found in workspace".to_string())
    }

    fn normalize_load_path(&self, path: &Path) -> PathBuf {
        normalize_virtual_path(path).unwrap_or_else(|_| path.to_path_buf())
    }
}

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

pub fn parse_song_from_virtual_workspace(
    entry_path: &str,
    active_path: &str,
    files: &HashMap<String, String>,
) -> Result<Song> {
    let entry_path =
        normalize_virtual_path(Path::new(entry_path)).map_err(|err| miette::miette!(err))?;
    let active_path =
        normalize_virtual_path(Path::new(active_path)).map_err(|err| miette::miette!(err))?;
    let mut normalized_files = HashMap::new();
    for (path, source) in files {
        let path = normalize_virtual_path(Path::new(path)).map_err(|err| miette::miette!(err))?;
        if normalized_files
            .insert(path.clone(), source.clone())
            .is_some()
        {
            return Err(miette::miette!(
                "Duplicate workspace file path '{}'",
                path.display()
            ));
        }
    }
    if !normalized_files.contains_key(&entry_path) {
        return Err(miette::miette!(
            "Workspace entry path '{}' is missing",
            entry_path.display()
        ));
    }
    if !normalized_files.contains_key(&active_path) {
        return Err(miette::miette!(
            "Workspace active path '{}' is missing",
            active_path.display()
        ));
    }

    let source = normalized_files
        .get(&entry_path)
        .expect("entry path existence was validated")
        .clone();
    let resolver = VirtualSourceResolver {
        files: normalized_files,
    };
    let base_dir = entry_path.parent();
    parse_song_internal_with_resolver(source, base_dir, &HashMap::new(), &resolver)
        .map_err(|err| err.with_source_name_if_default(entry_path.display().to_string()))
        .map_err(Into::into)
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
    let resolver = FsSourceResolver;
    parse_song_internal_with_resolver(source, base_dir, fragment_overrides, &resolver)
}

fn parse_song_internal_with_resolver<R: SourceResolver + ?Sized>(
    source: String,
    base_dir: Option<&Path>,
    fragment_overrides: &HashMap<PathBuf, String>,
    resolver: &R,
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

    let libraries = template_library::load_template_libraries(
        &metadata.templates,
        base_dir,
        &source,
        resolver,
    )?;

    let manifest_calls = manifest::collect_fragment_calls(input, &source)?;

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
        manifest::parse_manifest(manifest::ManifestParseInput {
            source: &source,
            input,
            metadata,
            libraries,
            manifest_calls,
            base_dir,
            fragment_overrides,
            resolver,
        })
    }
}

fn normalize_virtual_path(path: &Path) -> std::result::Result<PathBuf, &'static str> {
    if path.as_os_str().is_empty() {
        return Err("Workspace paths must not be empty");
    }

    let raw = path.to_string_lossy();
    if raw.contains('\\') {
        return Err("Workspace paths must use `/` separators");
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => return Err("Workspace paths must not contain `.` components"),
            Component::ParentDir => {
                return Err("Workspace paths must not contain parent traversal")
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err("Workspace paths must be relative")
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err("Workspace paths must not be empty");
    }
    Ok(normalized)
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
