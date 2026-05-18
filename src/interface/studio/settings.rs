use crate::dsl::parser;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TrackHeader {
    pub(super) name: String,
    pub(super) channel: u8,
    pub(super) solo: bool,
    pub(super) muted: bool,
}

pub(super) fn toggle_loop_frontmatter(source: &str) -> std::result::Result<(String, bool), String> {
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();

    if !matches!(lines.first().map(|line| line.as_str()), Some("---")) {
        let source = if source.is_empty() {
            "---\nloop: true\n---\n".to_string()
        } else {
            format!("---\nloop: true\n---\n\n{}", source)
        };
        return Ok((source, true));
    }

    let Some(end_index) = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line == "---").then_some(index))
    else {
        return Err("Loop toggle failed: frontmatter block is not closed".to_string());
    };

    for line in lines.iter_mut().take(end_index).skip(1) {
        let trimmed = line.trim();
        if !trimmed.starts_with("loop:") {
            continue;
        }

        match trimmed {
            "loop: true" => {
                *line = "loop: false".to_string();
                return Ok((finish_source(lines), false));
            }
            "loop: false" => {
                *line = "loop: true".to_string();
                return Ok((finish_source(lines), true));
            }
            _ => {
                return Err(
                    "Loop toggle supports only simple `loop: true` or `loop: false`".to_string(),
                );
            }
        }
    }

    lines.insert(end_index, "loop: true".to_string());
    Ok((finish_source(lines), true))
}

pub(super) fn loop_range_for_bar_indices(
    source: &str,
    start_index: usize,
    end_index: usize,
) -> std::result::Result<String, String> {
    let song = parser::parse_song(source.to_string())
        .map_err(|e| format!("Cannot set loop range: {}", e))?;

    if song.metadata.unit == "beat" {
        let beats_per_bar = crate::validation::beats_per_unit("bar", &song.metadata.signature)
            .map_err(|message| format!("Cannot set loop range: {}", message))?;
        let start = start_index as f64 * beats_per_bar;
        let end = (end_index + 1) as f64 * beats_per_bar;
        Ok(format_loop_range_number(start, end))
    } else {
        Ok(format!("{}..{}", start_index, end_index + 1))
    }
}

fn format_loop_range_number(start: f64, end: f64) -> String {
    format!("{}..{}", format_loop_bound(start), format_loop_bound(end))
}

fn format_loop_bound(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{}", value as u64)
    } else {
        format!("{:.4}", value)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

pub(super) fn set_loop_range_frontmatter(
    source: &str,
    loop_range: &str,
) -> std::result::Result<String, String> {
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();

    if !matches!(lines.first().map(|line| line.as_str()), Some("---")) {
        let source = if source.is_empty() {
            format!("---\nloop: true\nloop_range: {}\n---\n", loop_range)
        } else {
            format!(
                "---\nloop: true\nloop_range: {}\n---\n\n{}",
                loop_range, source
            )
        };
        return Ok(source);
    }

    let Some(end_index) = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line == "---").then_some(index))
    else {
        return Err("Loop range failed: frontmatter block is not closed".to_string());
    };

    let mut loop_line = None;
    let mut loop_range_line = None;
    for (index, line) in lines.iter().enumerate().take(end_index).skip(1) {
        let trimmed = line.trim();
        if trimmed.starts_with("loop:") {
            match trimmed {
                "loop: true" | "loop: false" => loop_line = Some(index),
                _ => {
                    return Err(
                        "Loop range supports only simple `loop: true` or `loop: false`".to_string(),
                    );
                }
            }
        } else if trimmed.starts_with("loop_range:") {
            if trimmed == "loop_range:" {
                return Err("Loop range supports only simple `loop_range: start..end`".to_string());
            }
            loop_range_line = Some(index);
        }
    }

    if let Some(index) = loop_line {
        lines[index] = "loop: true".to_string();
    } else {
        lines.insert(end_index, "loop: true".to_string());
        if let Some(index) = loop_range_line.as_mut() {
            *index += 1;
        }
    }

    if let Some(index) = loop_range_line {
        lines[index] = format!("loop_range: {}", loop_range);
    } else {
        let insert_index = lines
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(index, line)| (line == "---").then_some(index))
            .unwrap_or(lines.len());
        lines.insert(insert_index, format!("loop_range: {}", loop_range));
    }

    Ok(finish_source(lines))
}

pub(super) fn clear_loop_settings_frontmatter(
    source: &str,
) -> std::result::Result<Option<String>, String> {
    if !matches!(source.lines().next(), Some("---")) {
        return Ok(None);
    }

    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();
    let Some(end_index) = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line == "---").then_some(index))
    else {
        return Err("Loop clear failed: frontmatter block is not closed".to_string());
    };

    let mut changed = false;
    let mut remove_indices = Vec::new();
    for (index, line) in lines.iter().enumerate().take(end_index).skip(1) {
        let trimmed = line.trim();
        if trimmed.starts_with("loop:") {
            match trimmed {
                "loop: true" => {
                    changed = true;
                    remove_indices.push(index);
                }
                "loop: false" => {
                    remove_indices.push(index);
                }
                _ => {
                    return Err(
                        "Loop clear supports only simple `loop: true` or `loop: false`".to_string(),
                    );
                }
            }
        } else if trimmed.starts_with("loop_range:") {
            if trimmed == "loop_range:" {
                return Err("Loop clear supports only simple `loop_range: start..end`".to_string());
            }
            changed = true;
            remove_indices.push(index);
        }
    }

    if remove_indices.is_empty() {
        return Ok(None);
    }

    remove_indices.sort_unstable_by(|left, right| right.cmp(left));
    for index in remove_indices {
        lines.remove(index);
    }

    if !changed {
        return Ok(None);
    }
    let Some(end_index) = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line == "---").then_some(index))
    else {
        return Err("Loop clear failed: frontmatter block is not closed".to_string());
    };
    if lines[1..end_index]
        .iter()
        .all(|line| line.trim().is_empty())
    {
        lines.drain(0..=end_index);
        if matches!(lines.first(), Some(line) if line.trim().is_empty()) {
            lines.remove(0);
        }
    }
    Ok(Some(finish_source(lines)))
}

pub(super) fn parse_track_header_channel(line: &str) -> Option<u8> {
    parse_track_header(line)
        .and_then(|header| crate::validation::to_zero_based_channel(header.channel).ok())
}

pub(super) fn parse_track_header(line: &str) -> Option<TrackHeader> {
    let trimmed = line.trim();
    if trimmed.starts_with("##") || !trimmed.starts_with('#') {
        return None;
    }

    let (name, rest) = trimmed[1..].split_once(':')?;
    let rest = rest.trim_start();
    let channel = rest
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>()
        .parse::<u8>()
        .ok()?;
    crate::validation::to_zero_based_channel(channel).ok()?;
    let remainder = rest
        .chars()
        .skip_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    let mut solo = false;
    let mut muted = false;
    for flag in remainder.split_whitespace() {
        match flag {
            "s" => solo = true,
            "x" => muted = true,
            _ => return None,
        }
    }

    Some(TrackHeader {
        name: name.trim().to_string(),
        channel,
        solo,
        muted,
    })
}

pub(super) fn format_track_header(header: &TrackHeader) -> String {
    let mut out = format!("# {}: {}", header.name, header.channel);
    if header.solo {
        out.push_str(" s");
    }
    if header.muted {
        out.push_str(" x");
    }
    out
}

fn finish_source(lines: Vec<String>) -> String {
    let mut source = lines.join("\n");
    source.push('\n');
    source
}

#[cfg(test)]
mod tests {
    use super::{
        clear_loop_settings_frontmatter, format_track_header, loop_range_for_bar_indices,
        parse_track_header, parse_track_header_channel, set_loop_range_frontmatter,
        toggle_loop_frontmatter, TrackHeader,
    };

    #[test]
    fn toggle_loop_adds_frontmatter_when_missing() {
        let (source, enabled) = toggle_loop_frontmatter("# Piano: 1\nC4 | ^ |\n").unwrap();
        assert!(enabled);
        assert_eq!(source, "---\nloop: true\n---\n\n# Piano: 1\nC4 | ^ |\n");
    }

    #[test]
    fn toggle_loop_adds_key_to_existing_frontmatter() {
        let (source, enabled) =
            toggle_loop_frontmatter("---\nbpm: 100\n---\n# Piano: 1\n").unwrap();
        assert!(enabled);
        assert_eq!(source, "---\nbpm: 100\nloop: true\n---\n# Piano: 1\n");
    }

    #[test]
    fn toggle_loop_turns_on_and_off() {
        let (source, enabled) =
            toggle_loop_frontmatter("---\nloop: false\n---\n# Piano: 1\n").unwrap();
        assert!(enabled);
        assert_eq!(source, "---\nloop: true\n---\n# Piano: 1\n");

        let (source, enabled) = toggle_loop_frontmatter(&source).unwrap();
        assert!(!enabled);
        assert_eq!(source, "---\nloop: false\n---\n# Piano: 1\n");
    }

    #[test]
    fn toggle_loop_rejects_non_simple_loop_value() {
        let err = toggle_loop_frontmatter("---\nloop:\n  enabled: true\n---\n").unwrap_err();
        assert_eq!(
            err,
            "Loop toggle supports only simple `loop: true` or `loop: false`"
        );
    }

    #[test]
    fn parse_track_header_channel_returns_zero_based_channel() {
        assert_eq!(parse_track_header_channel("# Piano: 2"), Some(1));
        assert_eq!(parse_track_header_channel("# Drums: 10 x"), Some(9));
        assert_eq!(parse_track_header_channel("# Bass: 2 s x"), Some(1));
        assert_eq!(parse_track_header_channel("## sound 1"), None);
        assert_eq!(parse_track_header_channel("# Invalid: 17"), None);
    }

    #[test]
    fn parse_track_header_reads_name_channel_and_mute() {
        let header = parse_track_header("# Drums: 10 x").unwrap();
        assert_eq!(
            header,
            TrackHeader {
                name: "Drums".to_string(),
                channel: 10,
                solo: false,
                muted: true,
            }
        );
    }

    #[test]
    fn parse_track_header_reads_solo_and_rejects_unknown_flags() {
        let header = parse_track_header("# Bass: 2 x s").unwrap();
        assert_eq!(
            header,
            TrackHeader {
                name: "Bass".to_string(),
                channel: 2,
                solo: true,
                muted: true,
            }
        );
        assert_eq!(parse_track_header("# Bass: 2 z"), None);
    }

    #[test]
    fn format_track_header_emits_canonical_spacing() {
        let header = TrackHeader {
            name: "Piano".to_string(),
            channel: 1,
            solo: false,
            muted: false,
        };
        assert_eq!(format_track_header(&header), "# Piano: 1");

        let solo = TrackHeader {
            solo: true,
            ..header.clone()
        };
        assert_eq!(format_track_header(&solo), "# Piano: 1 s");

        let muted = TrackHeader {
            muted: true,
            ..header.clone()
        };
        assert_eq!(format_track_header(&muted), "# Piano: 1 x");

        let both = TrackHeader {
            solo: true,
            muted: true,
            ..header
        };
        assert_eq!(format_track_header(&both), "# Piano: 1 s x");
    }

    #[test]
    fn set_loop_range_adds_frontmatter_when_missing() {
        let source = set_loop_range_frontmatter("# Piano: 1\nC4 | ^ |\n", "0..1").unwrap();
        assert_eq!(
            source,
            "---\nloop: true\nloop_range: 0..1\n---\n\n# Piano: 1\nC4 | ^ |\n"
        );
    }

    #[test]
    fn set_loop_range_updates_existing_scalars() {
        let source =
            set_loop_range_frontmatter("---\nloop: false\nloop_range: 2..3\n---\n", "0..2")
                .unwrap();
        assert_eq!(source, "---\nloop: true\nloop_range: 0..2\n---\n");
    }

    #[test]
    fn clear_loop_settings_removes_loop_and_loop_range() {
        let source =
            clear_loop_settings_frontmatter("---\nbpm: 100\nloop: true\nloop_range: 0..2\n---\n")
                .unwrap()
                .unwrap();
        assert_eq!(source, "---\nbpm: 100\n---\n");
    }

    #[test]
    fn clear_loop_settings_removes_empty_frontmatter_block() {
        let source = clear_loop_settings_frontmatter("---\nloop: true\n---\n\n# Piano: 1\n")
            .unwrap()
            .unwrap();
        assert_eq!(source, "# Piano: 1\n");
    }

    #[test]
    fn clear_loop_settings_is_noop_without_enabled_loop_settings() {
        assert_eq!(
            clear_loop_settings_frontmatter("---\nloop: false\n---\n").unwrap(),
            None
        );
        assert_eq!(
            clear_loop_settings_frontmatter("# Piano: 1\n").unwrap(),
            None
        );
    }

    #[test]
    fn loop_range_uses_bar_unit_indices() {
        let range =
            loop_range_for_bar_indices("---\nunit: bar\n---\n# Piano: 1\nC4 | ^ | ^ |\n", 1, 2)
                .unwrap();
        assert_eq!(range, "1..3");
    }

    #[test]
    fn loop_range_converts_to_beats_for_beat_unit() {
        let range = loop_range_for_bar_indices(
            "---\nunit: beat\nsignature: 3/4\n---\n# Piano: 1\nC4 | ^ | ^ |\n",
            1,
            2,
        )
        .unwrap();
        assert_eq!(range, "3..9");
    }
}
