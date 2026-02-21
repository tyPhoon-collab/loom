mod core;
pub mod parser;

use crate::dsl::parser::ParsedLine;
use std::fmt::Write;

pub fn format_string(input: &str) -> String {
    let lines = parser::parse_for_formatting(input);
    let mut output = String::new();

    let mut elements: Vec<OutputElement> = Vec::new();
    let mut current_data: Vec<ParsedLine> = Vec::new();

    for line in lines {
        match line {
            ParsedLine::Pattern { .. } | ParsedLine::Modifier { .. } => {
                current_data.push(line);
            }
            ParsedLine::Empty => {
                // Explicitly ignore existing empty lines; we will regenerate them
                continue;
            }
            _ => {
                if !current_data.is_empty() {
                    elements.push(OutputElement::Data(current_data));
                    current_data = Vec::new();
                }
                elements.push(OutputElement::Meta(line));
            }
        }
    }
    if !current_data.is_empty() {
        elements.push(OutputElement::Data(current_data));
    }

    // Now write elements with mandatory empty line between them
    for (i, element) in elements.iter().enumerate() {
        if i > 0 {
            output.push('\n'); // One empty line between islands
        }

        let content = match element {
            OutputElement::Data(data_lines) => {
                let refs: Vec<&ParsedLine> = data_lines.iter().collect();
                core::format_patterns(&refs)
            }
            OutputElement::Meta(meta_line) => format_meta_line(meta_line),
        };

        // Write each line, trim-ending it
        for line in content.lines() {
            writeln!(output, "{}", line.trim_end()).unwrap();
        }
    }

    // Ensure EOF newline, and if file is empty, it stays empty or one newline
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }

    output
}

enum OutputElement {
    Data(Vec<ParsedLine>),
    Meta(ParsedLine),
}

fn format_meta_line(line: &ParsedLine) -> String {
    let mut out = String::new();
    match line {
        ParsedLine::Frontmatter(s) => out.push_str(s),
        ParsedLine::TrackHeader {
            name,
            channel,
            muted,
        } => {
            let mute_str = if *muted { " x" } else { "" };
            write!(out, "# {}: {}{}", name, channel, mute_str).unwrap();
        }
        ParsedLine::Comment(s) => {
            let trimmed = s.trim();
            if trimmed.starts_with('>') {
                out.push_str(trimmed);
            } else {
                write!(out, "> {}", trimmed).unwrap();
            }
        }
        ParsedLine::TrackWrap => out.push_str("---"),
        _ => {} // Should not happen for Meta
    }
    out
}
