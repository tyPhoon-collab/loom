use std::fs;

pub fn build_dsl_symbols() -> Result<String, String> {
    let src = fs::read_to_string("src/dsl/syntax.rs")
        .map_err(|e| format!("failed to read src/dsl/syntax.rs: {}", e))?;

    let mut out = String::new();
    out.push_str("## Symbol Table (Auto)\n\n");

    let mut pending_doc: Option<String> = None;
    for line in src.lines() {
        let trimmed = line.trim();
        if let Some(doc) = parse_doc_attr(trimmed) {
            pending_doc = Some(doc);
            continue;
        }
        if let Some((name, value)) = parse_symbol_mapping(trimmed) {
            out.push_str("- `");
            out.push_str(name);
            out.push_str("` => `");
            out.push_str(value);
            out.push_str("`");
            if let Some(doc) = pending_doc.take() {
                out.push_str(" - ");
                out.push_str(&doc);
            }
            out.push('\n');
        }
    }
    Ok(out)
}

fn parse_doc_attr(line: &str) -> Option<String> {
    let prefix = "#[doc = \"";
    if !line.starts_with(prefix) {
        return None;
    }
    let body = &line[prefix.len()..];
    let end = body.find("\"]")?;
    Some(body[..end].to_string())
}

fn parse_symbol_mapping(line: &str) -> Option<(&str, &str)> {
    let arrow = line.find("=>")?;
    let name = line[..arrow].trim().trim_end_matches(',');
    let value_raw = line[arrow + 2..].trim().trim_end_matches(',');
    let value = value_raw.strip_prefix('"')?.strip_suffix('"')?;
    Some((name, value))
}
