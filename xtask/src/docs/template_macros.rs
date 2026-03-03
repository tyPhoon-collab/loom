use std::fs;

pub fn build_template_macros() -> Result<String, String> {
    let src = fs::read_to_string("src/dsl/token.rs")
        .map_err(|e| format!("failed to read src/dsl/token.rs: {}", e))?;
    let macros = parse_template_macro_variants(&src);

    let mut out = String::new();
    out.push_str("## Template Macros (Auto)\n\n");
    for m in macros {
        let dsl = match m.as_str() {
            "Rev" => "rev",
            "Arp" => "arp",
            "Strum" => "strum",
            v if v.starts_with("Vel(") => "vel:N",
            v if v.starts_with("Pan(") => "pan:N",
            _ => m.as_str(),
        };
        out.push_str("- `");
        out.push_str(dsl);
        out.push_str("`\n");
    }
    Ok(out)
}

fn parse_template_macro_variants(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_enum = false;
    for line in src.lines() {
        let t = line.trim();
        if t.starts_with("pub enum TemplateMacro") {
            in_enum = true;
            continue;
        }
        if in_enum && t.starts_with('}') {
            break;
        }
        if !in_enum || t.is_empty() || t.starts_with("//") {
            continue;
        }
        let variant = t
            .trim_end_matches(',')
            .split_whitespace()
            .next()
            .unwrap_or(t);
        out.push(variant.to_string());
    }
    out
}
