use std::collections::BTreeSet;
use std::fs;

pub fn build_diagnostic_codes() -> Result<String, String> {
    let sources = ["src/dsl/error.rs", "src/compiler/error.rs"];
    let mut parser_codes = BTreeSet::new();
    let mut compiler_codes = BTreeSet::new();

    for src in sources {
        let content =
            fs::read_to_string(src).map_err(|e| format!("failed to read {}: {}", src, e))?;
        for line in content.lines() {
            if let Some(code) = extract_diagnostic_code(line) {
                if code.starts_with("loom::parser::") {
                    parser_codes.insert(code.to_string());
                } else if code.starts_with("loom::compiler::") {
                    compiler_codes.insert(code.to_string());
                }
            }
        }
    }

    let mut out = String::new();
    out.push_str("## Diagnostic Codes\n\n");
    out.push_str("### Parser\n\n");
    for code in parser_codes {
        out.push_str("- `");
        out.push_str(&code);
        out.push_str("`\n");
    }
    out.push('\n');
    out.push_str("### Compiler\n\n");
    for code in compiler_codes {
        out.push_str("- `");
        out.push_str(&code);
        out.push_str("`\n");
    }
    Ok(out)
}

fn extract_diagnostic_code(line: &str) -> Option<&str> {
    let needle = "#[diagnostic(code(";
    let start = line.find(needle)?;
    let rest = &line[start + needle.len()..];
    let end = rest.find("))]")?;
    Some(rest[..end].trim())
}
