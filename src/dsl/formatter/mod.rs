mod core;
pub mod parser;

use crate::dsl::parser::ParsedLine;
use crate::dsl::syntax::Symbol;
use miette::Result;
use std::fmt::Write;

pub fn format_string(input: &str) -> Result<String> {
    let lines = parser::parse_for_formatting(input)?;
    let mut output = String::new();

    let mut elements: Vec<OutputElement> = Vec::new();
    let mut current_data: Vec<ParsedLine> = Vec::new();

    for line in lines {
        match line {
            ParsedLine::Pattern { .. }
            | ParsedLine::Modifier { .. }
            | ParsedLine::TemplateCalls(_) => {
                current_data.push(line);
            }
            ParsedLine::Empty => {
                // Explicitly ignore existing empty lines; we will regenerate them
                continue;
            }
            ParsedLine::TrackWrap | ParsedLine::TemplateHeader { .. } => {
                if !current_data.is_empty() {
                    elements.push(OutputElement::Data(current_data));
                    current_data = Vec::new();
                }
                elements.push(OutputElement::Meta(line));
            }
            ParsedLine::Comment(_) => {
                if !current_data.is_empty() {
                    elements.push(OutputElement::Data(current_data));
                    current_data = Vec::new();
                }
                match elements.last_mut() {
                    Some(OutputElement::Comments(comments)) => {
                        comments.push(line);
                    }
                    _ => {
                        elements.push(OutputElement::Comments(vec![line]));
                    }
                }
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

    // Post-process: Keep TrackWrap only between two Data elements
    let mut final_elements = Vec::with_capacity(elements.len());
    let mut iter = elements.into_iter().peekable();
    while let Some(element) = iter.next() {
        if matches!(element, OutputElement::Meta(ParsedLine::TrackWrap)) {
            let prev_is_data = matches!(final_elements.last(), Some(OutputElement::Data(_)));
            let next_is_data = matches!(iter.peek(), Some(OutputElement::Data(_)));
            if !(prev_is_data && next_is_data) {
                continue;
            }
        }
        final_elements.push(element);
    }
    let elements = final_elements;

    // Now write elements with mandatory empty line between them
    for (i, element) in elements.iter().enumerate() {
        if i > 0 {
            output.push('\n'); // One empty line between islands
        }

        let content = match element {
            OutputElement::Data(data_lines) => {
                let refs: Vec<&ParsedLine> = data_lines.iter().collect();
                core::format_patterns(&refs)?
            }
            OutputElement::Meta(meta_line) => format_meta_line(meta_line),
            OutputElement::Comments(comment_lines) => {
                let mut out = String::new();
                for (j, c) in comment_lines.iter().enumerate() {
                    if j > 0 {
                        out.push('\n');
                    }
                    out.push_str(&format_meta_line(c));
                }
                out
            }
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

    Ok(output)
}

enum OutputElement {
    Data(Vec<ParsedLine>),
    Meta(ParsedLine),
    Comments(Vec<ParsedLine>),
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
            if trimmed.starts_with(Symbol::Comment.as_char()) {
                out.push_str(trimmed);
            } else {
                write!(out, "{} {}", Symbol::Comment, trimmed).unwrap();
            }
        }
        ParsedLine::TrackWrap => out.push_str(Symbol::TrackWrap.as_str()),
        ParsedLine::TemplateHeader { name } => {
            write!(out, "{} {}", Symbol::TrackHeader, Symbol::Template).unwrap();
            out.push_str(name);
        }
        ParsedLine::TemplateCalls(calls) => {
            for call in calls {
                write!(out, "{}", call).unwrap();
            }
        }
        _ => {} // Should not happen for Meta
    }
    out
}
