use std::fmt::Write as FmtWrite;
use std::str::FromStr;

#[derive(Debug)]
pub enum RawLine {
    Frontmatter(String),
    TrackHeader(String), // "# Name: Channel"
    Pattern(PatternLine),
    Comment(String),
    Empty,
    Other(String), // Fallback
}

#[derive(Debug)]
pub struct PatternLine {
    pub key: String,
    pub blocks: Vec<Vec<String>>,         // [Block][TokenString]
    pub trailing_comment: Option<String>, // Inline comment like "kick |...| > comment"
}

/// Parse the entire source into a list of RawLines for formatting.
/// We use a custom lighter parser here because we need to preserve comments and structure,
/// which the main parser might discard or normalize too much.
pub fn parse_for_formatting(input: &str) -> Vec<RawLine> {
    let mut lines = Vec::new();
    let mut in_frontmatter = false;
    let mut frontmatter_buffer = String::new();

    for (i, line) in input.lines().enumerate() {
        let trimmed = line.trim();

        // Frontmatter handling
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
                lines.push(RawLine::Frontmatter(frontmatter_buffer.clone()));
                frontmatter_buffer.clear();
            }
            continue;
        }

        if trimmed.is_empty() {
            lines.push(RawLine::Empty);
            continue;
        }

        if trimmed.starts_with('>') {
            lines.push(RawLine::Comment(trimmed.to_string()));
            continue;
        }

        if trimmed.starts_with('#') {
            lines.push(RawLine::TrackHeader(trimmed.to_string()));
            continue;
        }

        // Try to parse as pattern: "key | ... | > comment?"
        if let Some(pattern_line) = parse_pattern_line_rough(trimmed) {
            lines.push(RawLine::Pattern(pattern_line));
        } else {
            lines.push(RawLine::Other(line.to_string()));
        }
    }

    lines
}

fn parse_pattern_line_rough(line: &str) -> Option<PatternLine> {
    // 1. Extract Comment
    let (content, comment) = if let Some(idx) = line.find('>') {
        let (c, m) = line.split_at(idx);
        (c.trim(), Some(m.trim().to_string()))
    } else {
        (line, None)
    };

    // 2. Extract Key
    let pipe_start = content.find('|')?;
    let key = content[..pipe_start].trim().to_string();

    // 3. Extract Blocks
    let pipe_end = content.rfind('|')?;
    if pipe_start >= pipe_end {
        return None;
    }

    let blocks_content = &content[pipe_start + 1..pipe_end];
    let raw_blocks: Vec<&str> = blocks_content.split('|').collect();

    let mut blocks = Vec::new();
    for raw_block in raw_blocks {
        // Tokenize by splitting by whitespace, respecting groups `[...]`
        let tokens = tokenize_rough(raw_block);
        blocks.push(tokens);
    }

    Some(PatternLine {
        key,
        blocks,
        trailing_comment: comment,
    })
}

fn tokenize_rough(block_str: &str) -> Vec<String> {
    // Split by whitespace AND special characters (^, ., -) to ensure spacing.
    // Keep `[...]` groups together.

    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_group = false;

    // Use peeking iterator or simple state machine
    let chars: Vec<char> = block_str.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];

        if in_group {
            if c == ']' {
                current.push(c);
                tokens.push(current.trim().to_string());
                current.clear();
                in_group = false;
            } else {
                current.push(c);
            }
            i += 1;
            continue;
        }

        if c == '[' {
            if !current.trim().is_empty() {
                tokens.push(current.trim().to_string());
                current.clear();
            }
            current.push(c);
            in_group = true;
            i += 1;
            continue;
        }

        if c.is_whitespace() {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
            i += 1;
            continue;
        }

        // Special characters: ^, ., - should be separate tokens?
        // Yes, to force spacing: "c3 | ^.. |" -> "c3 | ^ . . |"
        if c == '^' || c == '.' || c == '-' {
            if !current.is_empty() {
                tokens.push(current.clone());
                current.clear();
            }
            tokens.push(c.to_string());
            i += 1;
            continue;
        }

        // Other characters (e.g. part of a longer token if any? unlikely in loom outside of groups)
        // But let's accumulate them just in case
        current.push(c);
        i += 1;
    }

    if !current.trim().is_empty() {
        tokens.push(current.trim().to_string());
    }

    tokens
}

pub fn format_string(input: &str) -> String {
    let lines = parse_for_formatting(input);
    let mut output = String::new();

    let mut pattern_buffer: Vec<&PatternLine> = Vec::new();

    for line in &lines {
        match line {
            RawLine::Pattern(p) => {
                pattern_buffer.push(p);
            }
            _ => {
                // If we have buffered patterns, flush them formatted
                if !pattern_buffer.is_empty() {
                    output.push_str(&format_patterns(&pattern_buffer));
                    pattern_buffer.clear();
                }

                // Print the current non-pattern line
                match line {
                    RawLine::Frontmatter(s) => output.push_str(s), // already has newline usually?
                    RawLine::TrackHeader(s) => {
                        writeln!(output, "{}", s).unwrap();
                    }
                    RawLine::Comment(s) => {
                        writeln!(output, "{}", s).unwrap();
                    }
                    RawLine::Empty => {
                        writeln!(output).unwrap();
                    }
                    RawLine::Other(s) => {
                        writeln!(output, "{}", s).unwrap();
                    }
                    RawLine::Pattern(_) => unreachable!(),
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

fn format_patterns(patterns: &[&PatternLine]) -> String {
    if patterns.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    let mut patterns = patterns.to_vec();

    // Sort by pitch (Descending: High -> Low)
    // We need to parse keys to Notes to compare them.
    // If parsing fails, we treat it as lowest priority (or keep relative order?)
    // Let's use a stable sort with a cached key.
    patterns.sort_by(|a, b| {
        let note_a = crate::note::Note::from_str(&a.key);
        let note_b = crate::note::Note::from_str(&b.key);

        match (note_a, note_b) {
            (Ok(na), Ok(nb)) => {
                let midi_a = na.to_midi();
                let midi_b = nb.to_midi();
                // Descending order (b.cmp(a))
                midi_b.cmp(&midi_a)
            }
            (Ok(_), Err(_)) => std::cmp::Ordering::Less, // Valid notes come first (top)
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
            (Err(_), Err(_)) => std::cmp::Ordering::Equal,
        }
    });

    // 1. Calculate max key width
    // Use `patterns` (the sorted vector) instead of the argument slice which is shadowed/moved?
    // Actually the argument `patterns` is a slice `&[&PatternLine]`.
    // `patterns.to_vec()` creates `Vec<&PatternLine>`.
    // We sort this new Vec.
    // Then we use this sorted Vec for iteration.

    let max_key_width = patterns.iter().map(|p| p.key.len()).max().unwrap_or(0);

    // 2. Logic to align blocks and tokens
    // Calculate max width for each token column across all patterns
    let max_blocks = patterns.iter().map(|p| p.blocks.len()).max().unwrap_or(0);

    // widths[block_index][token_index] = max_width
    let mut block_token_widths: Vec<Vec<usize>> = vec![Vec::new(); max_blocks];

    for p in &patterns {
        for (b_idx, block) in p.blocks.iter().enumerate() {
            if b_idx >= max_blocks {
                break;
            }

            for (t_idx, token) in block.iter().enumerate() {
                if t_idx >= block_token_widths[b_idx].len() {
                    block_token_widths[b_idx].push(0);
                }
                if token.len() > block_token_widths[b_idx][t_idx] {
                    block_token_widths[b_idx][t_idx] = token.len();
                }
            }
        }
    }

    // 3. Print
    for p in patterns {
        // Key
        write!(out, "{:width$} |", p.key, width = max_key_width).unwrap();

        for (b_idx, block) in p.blocks.iter().enumerate() {
            let token_widths = &block_token_widths[b_idx];

            for (t_idx, width) in token_widths.iter().enumerate() {
                // Get token if exists
                let token_str = block.get(t_idx).map(|s| s.as_str()).unwrap_or("");

                // Left-align with leading space
                write!(out, " {:width$}", token_str, width = width).unwrap();
            }

            write!(out, " |").unwrap();
        }

        if let Some(comment) = &p.trailing_comment {
            write!(out, " {}", comment).unwrap();
        }

        writeln!(out).unwrap();
    }

    out
}
