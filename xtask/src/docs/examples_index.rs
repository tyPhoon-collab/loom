use super::util::list_loom_files;

pub fn build_examples_index() -> Result<String, String> {
    let mut names = list_loom_files("examples")?;
    names.sort();

    let mut feature = Vec::new();
    let mut musical = Vec::new();
    let mut other = Vec::new();

    for n in names {
        if n.starts_with("feature-") {
            feature.push(n);
        } else if n.starts_with("drums-") || n.starts_with("melody-") || n.starts_with("chords-") {
            musical.push(n);
        } else {
            other.push(n);
        }
    }

    let mut out = String::new();
    out.push_str("## Example Files (Auto)\n\n");
    out.push_str("### Feature\n\n");
    for n in feature {
        out.push_str("- `examples/");
        out.push_str(&n);
        out.push_str("`\n");
    }
    out.push('\n');
    out.push_str("### Musical\n\n");
    for n in musical {
        out.push_str("- `examples/");
        out.push_str(&n);
        out.push_str("`\n");
    }
    out.push('\n');
    out.push_str("### Other\n\n");
    for n in other {
        out.push_str("- `examples/");
        out.push_str(&n);
        out.push_str("`\n");
    }
    Ok(out)
}
