use std::fs;

use super::util::list_loom_files_flat;

pub fn build_error_fixture_samples() -> Result<String, String> {
    let mut names = list_loom_files_flat("tests/fixtures/errors/input")?;
    names.sort();

    let mut out = String::new();
    out.push_str("## Error Fixtures (Auto)\n\n");
    out.push_str("### Index\n\n");
    for n in &names {
        out.push_str("- `");
        out.push_str(n);
        out.push_str("`\n");
    }
    out.push('\n');
    out.push_str("### Samples\n\n");

    for n in names {
        let path = format!("tests/fixtures/errors/input/{}", n);
        let body =
            fs::read_to_string(&path).map_err(|e| format!("failed to read {}: {}", path, e))?;
        out.push_str("#### `");
        out.push_str(&n);
        out.push_str("`\n\n");
        out.push_str("````loom\n");
        out.push_str(body.trim_end());
        out.push('\n');
        out.push_str("````\n\n");
    }
    Ok(out)
}
