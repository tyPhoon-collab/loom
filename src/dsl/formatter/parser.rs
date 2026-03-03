use crate::dsl::error::ParseError;
use crate::dsl::parser::{self, ParsedLine};
use crate::dsl::syntax::Symbol;

/// Parse the entire source into a list of ParsedLine for formatting.
pub fn parse_for_formatting(input: &str) -> Result<Vec<ParsedLine>, ParseError> {
    let mut lines = Vec::new();
    let mut in_frontmatter = false;
    let mut frontmatter_buffer = String::new();

    for (i, line) in input.lines().enumerate() {
        let trimmed = line.trim();

        // Frontmatter handling (Manual handling as parser might expect full string)
        if i == 0 && trimmed == Symbol::TrackWrap.as_str() {
            in_frontmatter = true;
            frontmatter_buffer.push_str(line);
            frontmatter_buffer.push('\n');
            continue;
        }
        if in_frontmatter {
            frontmatter_buffer.push_str(line);
            frontmatter_buffer.push('\n');
            if trimmed == Symbol::TrackWrap.as_str() {
                in_frontmatter = false;
                lines.push(ParsedLine::Frontmatter(frontmatter_buffer.clone()));
                frontmatter_buffer.clear();
            }
            continue;
        }

        // Use the strict parser for each line
        match parser::parse_line_entry(trimmed) {
            Ok((_, parsed)) => lines.push(parsed),
            Err(nom::Err::Error(e)) | Err(nom::Err::Failure(e)) => {
                return Err(ParseError::from_nom(line, input, format!("{:?}", e.code)));
            }
            Err(_) => return Err(ParseError::from_nom(line, input, "Incomplete".to_string())),
        }
    }

    if in_frontmatter {
        return Err(ParseError::from_yaml(
            input,
            input,
            "Unclosed Frontmatter. Missing closing '---'".to_string(),
        ));
    }

    Ok(lines)
}
