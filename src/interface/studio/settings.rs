use crate::dsl::token::Frontmatter;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TrackHeader {
    pub(super) name: String,
    pub(super) channel: u8,
    pub(super) solo: bool,
    pub(super) muted: bool,
}

pub(super) fn set_bpm_frontmatter(source: &str, bpm: u32) -> std::result::Result<String, String> {
    if !(1..=999).contains(&bpm) {
        return Err("BPM must be 1..999".to_string());
    }
    set_simple_frontmatter_value(source, "bpm", &bpm.to_string(), "BPM")
}

pub(super) fn set_loop_enabled_frontmatter(
    source: &str,
    enabled: bool,
) -> std::result::Result<String, String> {
    set_simple_frontmatter_value(
        source,
        "loop",
        if enabled { "true" } else { "false" },
        "Loop",
    )
}

pub(super) fn loop_range_from_bounds(start: f64, end: f64) -> std::result::Result<String, String> {
    if !start.is_finite() || !end.is_finite() || start < 0.0 {
        return Err("Loop range bounds must be non-negative numbers".to_string());
    }
    if end <= start {
        return Err("Loop range end must be greater than start".to_string());
    }
    Ok(format_loop_range_number(start, end))
}

pub(super) fn loop_range_for_bar_indices(
    source: &str,
    start_index: usize,
    end_index: usize,
) -> std::result::Result<String, String> {
    let metadata =
        parse_metadata_only(source).map_err(|e| format!("Cannot set loop range: {}", e))?;

    if metadata.unit == "beat" {
        let beats_per_bar = crate::validation::beats_per_unit("bar", &metadata.signature)
            .map_err(|message| format!("Cannot set loop range: {}", message))?;
        let start = start_index as f64 * beats_per_bar;
        let end = (end_index + 1) as f64 * beats_per_bar;
        Ok(format_loop_range_number(start, end))
    } else {
        Ok(format!("{}..{}", start_index, end_index + 1))
    }
}

fn parse_metadata_only(source: &str) -> std::result::Result<Frontmatter, String> {
    if source.starts_with("---") {
        crate::dsl::parser::nom_parsers::parse_frontmatter(source)
            .map(|(_, metadata)| metadata)
            .map_err(|_| "Invalid Frontmatter YAML".to_string())
    } else {
        Ok(Frontmatter::default())
    }
}

fn set_simple_frontmatter_value(
    source: &str,
    key: &str,
    value: &str,
    label: &str,
) -> std::result::Result<String, String> {
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();
    let line = format!("{}: {}", key, value);

    if !matches!(lines.first().map(|line| line.as_str()), Some("---")) {
        let source = if source.is_empty() {
            format!("---\n{}\n---\n", line)
        } else {
            format!("---\n{}\n---\n\n{}", line, source)
        };
        return Ok(source);
    }

    let Some(end_index) = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (line == "---").then_some(index))
    else {
        return Err(format!(
            "{} update failed: frontmatter block is not closed",
            label
        ));
    };

    for line_ref in lines.iter_mut().take(end_index).skip(1) {
        let trimmed = line_ref.trim();
        if !trimmed.starts_with(&format!("{}:", key)) {
            continue;
        }
        if trimmed == format!("{}:", key) {
            return Err(format!(
                "{} update supports only simple `{}: value`",
                label, key
            ));
        }
        *line_ref = line;
        return Ok(finish_source(lines));
    }

    lines.insert(end_index, line);
    Ok(finish_source(lines))
}

pub(super) fn track_bar_index_at(
    source: &str,
    target_row: usize,
    local_index: usize,
) -> std::result::Result<usize, String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut row = 0usize;

    if matches!(lines.first().copied(), Some("---")) {
        row = score_body_start_row(source)?;
    }

    let mut in_track = false;
    let mut in_template = false;
    let mut section_base = 0usize;
    let mut section_bar_count = 0usize;

    while row < lines.len() {
        let line = lines[row];
        let trimmed = line.trim();

        if parse_track_header(line).is_some() {
            in_track = true;
            in_template = false;
            section_base = 0;
            section_bar_count = 0;
            row += 1;
            continue;
        }

        if is_template_header(line) {
            in_track = false;
            in_template = true;
            row += 1;
            continue;
        }

        if trimmed == "---" {
            if in_track {
                section_base += section_bar_count;
                section_bar_count = 0;
            }
            row += 1;
            continue;
        }

        if row == target_row {
            if in_template || !in_track {
                return Err("Loop range needs bars in a track body".to_string());
            }

            let bar_count = bar_count_in_line(line);
            if local_index >= bar_count {
                return Err("Selected bar no longer exists".to_string());
            }
            return Ok(section_base + local_index);
        }

        if in_track {
            section_bar_count = section_bar_count.max(bar_count_in_line(line));
        }
        row += 1;
    }

    Err("Selected bar no longer exists".to_string())
}

pub(super) fn score_body_start_row(source: &str) -> std::result::Result<usize, String> {
    let lines: Vec<&str> = source.lines().collect();
    if !matches!(lines.first().copied(), Some("---")) {
        return Ok(0);
    }

    let Some(end_index) = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (*line == "---").then_some(index))
    else {
        return Err("Frontmatter block is not closed".to_string());
    };

    let mut row = end_index + 1;
    while row < lines.len() && lines[row].trim().is_empty() {
        row += 1;
    }
    Ok(row)
}

fn is_template_header(line: &str) -> bool {
    line.trim().starts_with("# @")
}

fn bar_count_in_line(line: &str) -> usize {
    let pipe_count = line.chars().filter(|&ch| ch == '|').count();
    pipe_count.saturating_sub(1)
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
        loop_range_from_bounds, parse_track_header, parse_track_header_channel,
        score_body_start_row, set_bpm_frontmatter, set_loop_enabled_frontmatter,
        set_loop_range_frontmatter, track_bar_index_at, TrackHeader,
    };

    #[test]
    fn set_bpm_adds_frontmatter_when_missing() {
        let source = set_bpm_frontmatter("# Piano: 1\n", 140).unwrap();
        assert_eq!(source, "---\nbpm: 140\n---\n\n# Piano: 1\n");
    }

    #[test]
    fn set_bpm_updates_existing_scalar() {
        let source = set_bpm_frontmatter("---\nbpm: 100\n---\n# Piano: 1\n", 140).unwrap();
        assert_eq!(source, "---\nbpm: 140\n---\n# Piano: 1\n");
    }

    #[test]
    fn set_bpm_rejects_complex_value() {
        let err = set_bpm_frontmatter("---\nbpm:\n  base: 120\n---\n", 140).unwrap_err();
        assert_eq!(err, "BPM update supports only simple `bpm: value`");
    }

    #[test]
    fn set_loop_enabled_preserves_loop_range() {
        let source =
            set_loop_enabled_frontmatter("---\nloop: true\nloop_range: 0..4\n---\n", false)
                .unwrap();
        assert_eq!(source, "---\nloop: false\nloop_range: 0..4\n---\n");
    }

    #[test]
    fn loop_range_from_bounds_formats_numbers() {
        assert_eq!(loop_range_from_bounds(0.0, 4.0).unwrap(), "0..4");
        assert_eq!(loop_range_from_bounds(0.5, 4.25).unwrap(), "0.5..4.25");
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

    #[test]
    fn track_bar_index_at_continues_across_track_wraps() {
        let source = "# Piano: 1\nseq | C4 | D4 |\n---\nseq | E4 | F4 |\n";
        assert_eq!(track_bar_index_at(source, 1, 0).unwrap(), 0);
        assert_eq!(track_bar_index_at(source, 1, 1).unwrap(), 1);
        assert_eq!(track_bar_index_at(source, 3, 0).unwrap(), 2);
        assert_eq!(track_bar_index_at(source, 3, 1).unwrap(), 3);
    }

    #[test]
    fn track_bar_index_at_uses_widest_line_in_previous_section() {
        let source = "# Drums: 10\nkick  | ^ | ^ |\nsnare | . |\n---\nkick  | ^ |\n";
        assert_eq!(track_bar_index_at(source, 4, 0).unwrap(), 2);
    }

    #[test]
    fn track_bar_index_at_rejects_template_bars() {
        let source = "# @riff\nseq | C4 | D4 |\n";
        assert_eq!(
            track_bar_index_at(source, 1, 0).unwrap_err(),
            "Loop range needs bars in a track body"
        );
    }

    #[test]
    fn score_body_start_row_accounts_for_frontmatter_gap() {
        assert_eq!(score_body_start_row("# Piano: 1\n").unwrap(), 0);
        assert_eq!(
            score_body_start_row("---\nloop: true\n---\n\n# Piano: 1\n").unwrap(),
            4
        );
        assert_eq!(
            score_body_start_row("---\nloop: true\n---\n# Piano: 1\n").unwrap(),
            3
        );
    }
}
