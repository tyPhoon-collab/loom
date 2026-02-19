use crate::dsl::parser::{self, ParsedLine};

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
                lines.push(ParsedLine::Comment(line.to_string()));
            }
        }
    }

    lines
}
