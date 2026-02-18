use crate::dsl::parser::{self, ParsedLine};
use std::fmt::Write as FmtWrite;
use std::str::FromStr;

/// Parse the entire source into a list of ParsedLine for formatting.
pub fn parse_for_formatting(input: &str) -> Vec<ParsedLine> {
    let mut lines = Vec::new();
    let mut in_frontmatter = false;
    let mut frontmatter_buffer = String::new();

    for (i, line) in input.lines().enumerate() {
        let trimmed = line.trim();

        // Frontmatter handling (Manual handling as parser might expect full string)
        if i == 0 && trimmed == "---" {
            in_frontmatter = true;
            frontmatter_buffer.push_str(line);
            frontmatter_buffer.push('\n');
            continue;
        }
        if in_frontmatter {
            frontmatter_buffer.push_str(line);
            frontmatter_buffer.push('\n');
            if trimmed == "---" {
                in_frontmatter = false;
                lines.push(ParsedLine::Frontmatter(frontmatter_buffer.clone()));
                frontmatter_buffer.clear();
            }
            continue;
        }

        // Use the strict parser for each line
        match parser::parse_line_entry(trimmed) {
            Ok((_, parsed)) => lines.push(parsed),
            Err(_) => {
                // Fallback for lines that don't match (shouldn't happen with strict parser unless syntax error)
                // For formatting, maybe we want to preserve as Comment or just skip?
                // Let's treat as empty or simple comment to preserve content?
                // Actually, if it's invalid, we might want to leave it as is.
                // But we don't have "Other" variant in ParsedLine anymore.
                // Let's assume it's a comment for now or just log.
                // Since this is a formatter, crashing or dropping lines is bad.
                // But we are refining the parser.
                // Let's use a "Comment" fallback for now if it doesn't parse.
                lines.push(ParsedLine::Comment(line.to_string()));
            }
        }
    }

    lines
}

pub fn format_string(input: &str) -> String {
    let lines = parse_for_formatting(input);
    let mut output = String::new();

    let mut pattern_buffer: Vec<&ParsedLine> = Vec::new();

    for line in &lines {
        match line {
            ParsedLine::Pattern { .. } => {
                pattern_buffer.push(line);
            }
            _ => {
                // If we have buffered patterns, flush them formatting
                if !pattern_buffer.is_empty() {
                    output.push_str(&format_patterns(&pattern_buffer));
                    pattern_buffer.clear();
                }

                // Print the current non-pattern line
                match line {
                    ParsedLine::Frontmatter(s) => output.push_str(s),
                    ParsedLine::TrackHeader { name, channel } => {
                        writeln!(output, "# {}: {}", name, channel).unwrap();
                    }
                    ParsedLine::Comment(s) => {
                        // Ensure comment starts with > if strictly parsed as comment,
                        // but if it was a fallback line, it might not.
                        // parse_comment consumes '>', so 's' content relies on that.
                        // But fallback puts full line.
                        if s.trim().starts_with('>') {
                            writeln!(output, "{}", s).unwrap();
                        } else {
                            // Reconstruct comment
                            writeln!(output, "> {}", s).unwrap();
                        }
                    }
                    ParsedLine::Empty => {
                        writeln!(output).unwrap();
                    }
                    ParsedLine::Pattern { .. } => unreachable!(),
                }
            }
        }
    }

    // Flush remaining
    if !pattern_buffer.is_empty() {
        output.push_str(&format_patterns(&pattern_buffer));
    }

    output
}

fn format_patterns(patterns: &[&ParsedLine]) -> String {
    if patterns.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    let mut patterns = patterns.to_vec();

    // Sort by pitch
    patterns.sort_by(|a, b| {
        let (key_a, key_b) = match (a, b) {
            (ParsedLine::Pattern { key: k1, .. }, ParsedLine::Pattern { key: k2, .. }) => (k1, k2),
            _ => return std::cmp::Ordering::Equal,
        };

        let parse_midi_max = |key: &str| -> Option<u8> {
            key.split(',')
                .filter_map(|s| crate::dsl::note::Note::from_str(s.trim()).ok())
                .map(|n| n.to_midi())
                .max()
        };

        match (parse_midi_max(key_a), parse_midi_max(key_b)) {
            (Some(max_a), Some(max_b)) => max_b.cmp(&max_a),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    // Calculate sorted keys and max width
    let mut sorted_keys = Vec::new();
    for p in &patterns {
        if let ParsedLine::Pattern { key, .. } = p {
            let mut ns = key
                .split(',')
                .filter_map(|s| {
                    let trimmed = s.trim();
                    crate::dsl::note::Note::from_str(trimmed)
                        .ok()
                        .map(|n| (n, trimmed.to_string()))
                })
                .collect::<Vec<_>>();

            if ns.is_empty() {
                sorted_keys.push(key.clone());
            } else {
                ns.sort_by(|(n1, _), (n2, _)| n1.to_midi().cmp(&n2.to_midi()));
                let sk = ns
                    .iter()
                    .map(|(_, original)| original.clone())
                    .collect::<Vec<_>>()
                    .join(",");
                sorted_keys.push(sk);
            }
        }
    }

    let max_key_width = sorted_keys.iter().map(|k| k.len()).max().unwrap_or(0);

    // Calculate block widths
    let max_blocks = patterns
        .iter()
        .map(|p| match p {
            ParsedLine::Pattern { blocks, .. } => blocks.len(),
            _ => 0,
        })
        .max()
        .unwrap_or(0);

    let mut block_token_widths: Vec<Vec<usize>> = vec![Vec::new(); max_blocks];

    for p in &patterns {
        if let ParsedLine::Pattern { blocks, .. } = p {
            for (b_idx, block) in blocks.iter().enumerate() {
                if b_idx >= max_blocks {
                    break;
                }

                for (t_idx, token) in block.tokens.iter().enumerate() {
                    if t_idx >= block_token_widths[b_idx].len() {
                        block_token_widths[b_idx].push(0);
                    }
                    let token_len = token.to_string().len();
                    if token_len > block_token_widths[b_idx][t_idx] {
                        block_token_widths[b_idx][t_idx] = token_len;
                    }
                }
            }
        }
    }

    // Print
    for (i, p) in patterns.iter().enumerate() {
        if let ParsedLine::Pattern {
            blocks,
            trailing_comment,
            ..
        } = p
        {
            let sorted_key = &sorted_keys[i];

            write!(out, "{:width$} |", sorted_key, width = max_key_width).unwrap();

            for (b_idx, block) in blocks.iter().enumerate() {
                let token_widths = &block_token_widths[b_idx];

                for (t_idx, width) in token_widths.iter().enumerate() {
                    let token_str = block
                        .tokens
                        .get(t_idx)
                        .map(|t| t.to_string())
                        .unwrap_or_default();
                    write!(out, " {:width$}", token_str, width = width).unwrap();
                }

                write!(out, " |").unwrap();
            }

            if let Some(comment) = trailing_comment {
                write!(out, " > {}", comment).unwrap();
            }

            writeln!(out).unwrap();
        }
    }

    out
}
