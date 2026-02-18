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
    // let mut patterns = patterns.to_vec(); // No need to clone reference list if we don't reorder it?
    // Ah, we sort it. So we need a mutable vector of references.
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
    // Calculate key widths
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
                    .map(|(n, _)| n.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                sorted_keys.push(sk);
            }
        }
    }

    let max_key_width = sorted_keys.iter().map(|k| k.len()).max().unwrap_or(0);

    // Calculate block widths
    // We need to handle blocks properly.
    // Each block has a start_bar.
    // We align blocks by index.
    // Block 0: bar formatted width + space + token formatted width...

    // Max blocks
    let max_blocks = patterns
        .iter()
        .map(|p| match p {
            ParsedLine::Pattern { blocks, .. } => blocks.len(),
            _ => 0,
        })
        .max()
        .unwrap_or(0);

    // For each block index, we need:
    // 1. Max width of the start bar (e.g. `|` is 1, `|:` is 2)
    // 2. Max width of each token in that block?
    // Actually, alignment is usually:
    // Key | Token Token | Token |
    // With `|:` it might be:
    // Key |: Token Token | Token |
    // Key |  Token Token | Token |
    // So the "Bar" column should be aligned too?
    // Let's assume yes.

    // Structure: [Block 0 Bar Width, Block 0 Tokens Widths...] ?
    // Simplification:
    // Column 0: Bar (max width amongst all lines for block 0)
    // Column 1..N: Tokens (max width for token i in block 0)

    // We need to know max tokens in block 0 across all lines too.

    let mut block_info = Vec::new(); // Vec of (bar_width, Vec<token_width>)

    for i in 0..max_blocks {
        let mut max_bar_width = 0;
        let mut max_tokens = 0;

        // First pass: find max bar width and max token count for this block index
        for p in &patterns {
            if let ParsedLine::Pattern { blocks, .. } = p {
                if let Some(block) = blocks.get(i) {
                    let w = block.start_bar.to_string().len();
                    if w > max_bar_width {
                        max_bar_width = w;
                    }
                    if block.tokens.len() > max_tokens {
                        max_tokens = block.tokens.len();
                    }
                }
            }
        }

        // Second pass: find max width for each token index
        let mut token_widths = vec![0; max_tokens];
        for p in &patterns {
            if let ParsedLine::Pattern { blocks, .. } = p {
                if let Some(block) = blocks.get(i) {
                    for (t_idx, t) in block.tokens.iter().enumerate() {
                        let s = t.to_string();
                        if s.len() > token_widths[t_idx] {
                            token_widths[t_idx] = s.len();
                        }
                    }
                }
            }
        }

        block_info.push((max_bar_width, token_widths));
    }

    // Print
    for (i, p) in patterns.iter().enumerate() {
        if let ParsedLine::Pattern {
            blocks,
            end_bar,
            trailing_comment,
            ..
        } = p
        {
            let sorted_key = &sorted_keys[i];

            // Print Key
            write!(out, "{:width$}", sorted_key, width = max_key_width).unwrap();

            // Print Blocks
            for (b_idx, block) in blocks.iter().enumerate() {
                // Space before bar
                write!(out, " ").unwrap();

                let (bar_w, token_ws) = &block_info[b_idx];
                let b_str = block.start_bar.to_string();

                // Print Bar (Right aligned or Left? Usually bars are left aligned or center?
                // `| ` vs `|:`. If max is 2, `|` should probably be `| ` or ` |`?
                // Sheet music: strict alignment.
                // Let's right align bar: ` |` vs `|:`.
                // Or left: `| ` vs `|:`.
                // If we have `|` and `|:`, left align makes sense?
                // `|:`
                // `| `
                write!(out, "{:width$}", b_str, width = bar_w).unwrap();

                // Print Tokens
                for (t_idx, token) in block.tokens.iter().enumerate() {
                    let w = token_ws[t_idx];
                    write!(out, " {:width$}", token.to_string(), width = w).unwrap();
                }
            }

            // Handle Closing Bar?
            // The parser does NOT currently store the final closing bar in a way that is associated with a block.
            // And `blocks` only hold `start_bar`.
            // So we effectively lose the final `|` or `:|` in the current `Vec<Block>` structure if we purely iterate blocks.
            // BUT, `parse_line_blocks` loop breaks when `next_bar` is not found.
            // Wait, if `parse_line_blocks` stops, it means the last thing parsed was tokens.
            // If the line ends with `|`, that `|` triggered the START of a new block?
            // No, `parse_line_blocks` Structure:
            // 1. Parse Bar (Start of Block 0)
            // 2. Parse Tokens (Content of Block 0)
            // 3. Parse Bar (End of Block 0 / Start of Block 1).
            // ...
            // If line is `| A | B |`:
            // 1. `|` (Start B0)
            // 2. `A`
            // 3. `|` (End B0 / Start B1). push Block0.
            // 4. `B`
            // 5. `|` (End B1 / Start B2). push Block1.
            // 6. Tokens empty? (If `|` is at end of line)
            // 7. Parse Bar -> Error (EOF).
            // Loop breaks.
            // We have Block0, Block1.
            // But Block1's `start_bar` is the middle `|`.
            // Where is the FINAL `|`?
            // It was consumed by `parse_bar` in step 5?
            // Step 5 parsed `|`. `current_bar` becomes `|`.
            // Step 6 `parse_block_tokens` parses empty?
            // `parse_block_tokens` uses `many0`, so it matches empty.
            // Then `parse_bar` checks for EOF?
            // If EOF, `parse_bar` fails.
            // So loop breaks.
            // But we have `current_bar` set to the final `|`!
            // But we didn't push a block for it because the loop broke?
            // Wait, if `parse_block_tokens` returns empty tokens, and then `parse_bar` fails (EOF),
            // We break.
            // So `blocks` has Block0, Block1.
            // But `current_bar` is holding the final `|`.
            // We need to return it!

            // Print Closing Bar
            write!(out, " {}", end_bar).unwrap();

            if let Some(comment) = trailing_comment {
                write!(out, " > {}", comment).unwrap();
            }

            writeln!(out).unwrap();
        }
    }

    out
}
