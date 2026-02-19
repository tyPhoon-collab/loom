pub mod core;
mod equal;
mod justify;
mod minimize;
pub mod parser;
mod time;

use crate::dsl::parser::ParsedLine;
pub use core::FormattingMode;
use std::fmt::Write;

pub fn format_string(input: &str) -> String {
    let lines = parser::parse_for_formatting(input);

    // Check for frontmatter to determine mode
    let mut mode = FormattingMode::Equal; // Default

    for line in &lines {
        if let ParsedLine::Frontmatter(s) = line {
            if let Ok(fm) = serde_yaml::from_str::<crate::dsl::token::Frontmatter>(s) {
                if let Some(f) = fm.formatter {
                    match f.to_lowercase().as_str() {
                        "minimize" => mode = FormattingMode::Minimize,
                        "justify" => mode = FormattingMode::Justify,
                        "equal" => mode = FormattingMode::Equal,
                        "time" => mode = FormattingMode::Time,
                        _ => {} // Unknown mode, stick to default
                    }
                }
            }
            break; // Only first frontmatter counts
        }
    }

    format_string_with_mode(input, mode)
}

pub fn format_string_with_mode(input: &str, mode: FormattingMode) -> String {
    let lines = parser::parse_for_formatting(input);
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
                    output.push_str(&format_patterns(&pattern_buffer, mode));
                    pattern_buffer.clear();
                }

                // Print the current non-pattern line
                match line {
                    ParsedLine::Frontmatter(s) => output.push_str(s),
                    ParsedLine::TrackHeader { name, channel } => {
                        writeln!(output, "# {}: {}", name, channel).unwrap();
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
                    ParsedLine::Pattern { .. } => unreachable!(),
                }
            }
        }
    }

    // Flush remaining
    if !pattern_buffer.is_empty() {
        output.push_str(&format_patterns(&pattern_buffer, mode));
    }

    output
}

fn format_patterns(patterns: &[&ParsedLine], mode: FormattingMode) -> String {
    match mode {
        FormattingMode::Minimize => minimize::format_patterns_minimize(patterns),
        FormattingMode::Justify => justify::format_patterns_justify(patterns),
        FormattingMode::Equal => equal::format_patterns_equal(patterns),
        FormattingMode::Time => time::format_patterns_time(patterns),
    }
}
