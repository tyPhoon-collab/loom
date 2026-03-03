use std::fs;

pub fn build_cli_reference() -> Result<String, String> {
    let src = fs::read_to_string("src/cli.rs")
        .map_err(|e| format!("failed to read src/cli.rs: {}", e))?;
    let commands = parse_cli_commands(&src);

    let mut out = String::new();
    out.push_str("## Commands (Auto)\n\n");
    for cmd in commands {
        out.push_str("- `loom ");
        out.push_str(&cmd.name);
        out.push_str("`");
        if let Some(desc) = cmd.desc {
            out.push_str(": ");
            out.push_str(&desc);
        }
        out.push('\n');
        for arg in cmd.args {
            out.push_str("  - `");
            out.push_str(&arg);
            out.push_str("`\n");
        }
    }
    Ok(out)
}

#[derive(Debug)]
struct CliCommand {
    name: String,
    desc: Option<String>,
    args: Vec<String>,
}

fn parse_cli_commands(src: &str) -> Vec<CliCommand> {
    let mut commands = Vec::new();
    let mut in_enum = false;
    let mut current_doc: Option<String> = None;
    let mut i = 0usize;
    let lines: Vec<&str> = src.lines().collect();

    while i < lines.len() {
        let t = lines[i].trim();
        if t.starts_with("pub enum Commands") {
            in_enum = true;
            i += 1;
            continue;
        }
        if in_enum && t == "}" {
            break;
        }
        if !in_enum {
            i += 1;
            continue;
        }

        if t.starts_with("///") {
            current_doc = Some(t.trim_start_matches("///").trim().to_string());
            i += 1;
            continue;
        }

        if t.is_empty() {
            i += 1;
            continue;
        }

        if let Some(name) = parse_variant_name(t) {
            let mut cmd = CliCommand {
                name: to_kebab_case(name),
                desc: current_doc.take(),
                args: Vec::new(),
            };

            if t.contains('{') && !t.contains('}') {
                let mut pending_long = false;
                let mut pending_short = false;
                i += 1;
                while i < lines.len() {
                    let inner = lines[i].trim();
                    if inner.starts_with("///") {
                        i += 1;
                        continue;
                    }
                    if inner.starts_with("#[arg(") {
                        pending_long |= inner.contains("long");
                        pending_short |= inner.contains("short");
                        i += 1;
                        continue;
                    }
                    if inner.starts_with('}') {
                        break;
                    }
                    if let Some(field) = parse_field_name(inner) {
                        let rendered = if pending_long && pending_short {
                            format!("-{}, --{}", short_name(field), field)
                        } else if pending_long {
                            format!("--{}", field)
                        } else if pending_short {
                            format!("-{}", short_name(field))
                        } else {
                            field.to_string()
                        };
                        cmd.args.push(rendered);
                        pending_long = false;
                        pending_short = false;
                    }
                    i += 1;
                }
            }
            commands.push(cmd);
        }
        i += 1;
    }

    commands
}

fn parse_variant_name(line: &str) -> Option<&str> {
    let first = line.split_whitespace().next()?;
    if !first.chars().next()?.is_ascii_uppercase() {
        return None;
    }
    Some(first.trim_end_matches('{').trim_end_matches(','))
}

fn parse_field_name(line: &str) -> Option<&str> {
    if line.starts_with('#') {
        return None;
    }
    let colon = line.find(':')?;
    Some(line[..colon].trim().trim_end_matches(','))
}

fn short_name(field: &str) -> char {
    field.chars().next().unwrap_or('x')
}

fn to_kebab_case(s: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in s.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if idx > 0 {
                out.push('-');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}
