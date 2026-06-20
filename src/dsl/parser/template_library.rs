use super::{parse_frontmatter, parse_line_entry, resolve_template_library_path, ParsedLine};
use crate::dsl::error::ParseError;
use crate::dsl::token::{Frontmatter, TemplateLibrary};
use miette::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(super) fn load_template_libraries(
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

    let mut builder = super::song_builder::SongBuilder::new(source);
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
