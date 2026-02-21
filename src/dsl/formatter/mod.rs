mod core;
pub mod parser;

use crate::dsl::parser::ParsedLine;
use std::fmt::Write;

pub fn format_string(input: &str) -> String {
    let lines = parser::parse_for_formatting(input);
    let mut output = String::new();

    let mut pattern_buffer: Vec<&ParsedLine> = Vec::new();

    for line in &lines {
        match line {
            ParsedLine::Pattern { .. } | ParsedLine::Modifier { .. } => {
                pattern_buffer.push(line);
            }
            _ => {
                // If we have buffered patterns, flush them formatting
                if !pattern_buffer.is_empty() {
                    output.push_str(&core::format_patterns(&pattern_buffer));
                    pattern_buffer.clear();
                }

                // Print the current non-pattern line
                match line {
                    ParsedLine::Frontmatter(s) => output.push_str(s),
                    ParsedLine::TrackHeader {
                        name,
                        channel,
                        muted,
                    } => {
                        let mute_str = if *muted { " x" } else { "" };
                        writeln!(output, "# {}: {}{}", name, channel, mute_str).unwrap();
                    }
                    ParsedLine::Comment(s) => {
                        // Ensure comment starts with > if strictly parsed as comment
                        if s.trim().starts_with('>') {
                            writeln!(output, "{}", s).unwrap();
                        } else {
                            writeln!(output, "> {}", s).unwrap();
                        }
                    }
                    ParsedLine::Empty => {
                        writeln!(output).unwrap();
                    }
                    ParsedLine::TrackWrap => {
                        writeln!(output, "---").unwrap();
                    }
                    ParsedLine::Pattern { .. } | ParsedLine::Modifier { .. } => unreachable!(),
                }
            }
        }
    }

    // Flush remaining
    if !pattern_buffer.is_empty() {
        output.push_str(&core::format_patterns(&pattern_buffer));
    }

    output
}
