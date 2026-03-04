use std::fs;

use super::util::list_loom_files_recursive;

pub fn build_examples_overview() -> Result<String, String> {
    let starter = list_category_files("starter")?;
    let musical = list_category_files("musical")?;
    let live = list_category_files("live-coding")?;

    let mut out = String::new();
    out.push_str("## Category Summary (Auto)\n\n");
    out.push_str("| Category | Files |\n");
    out.push_str("| --- | ---: |\n");
    out.push_str(&format!(
        "| [Starter](/examples/starter) | {} |\n",
        starter.len()
    ));
    out.push_str(&format!(
        "| [Musical](/examples/musical) | {} |\n",
        musical.len()
    ));
    out.push_str(&format!(
        "| [Live Coding](/examples/live-coding) | {} |\n",
        live.len()
    ));
    Ok(out)
}

pub fn build_examples_category(
    title: &str,
    category: &str,
    include_samples: bool,
) -> Result<String, String> {
    let files = list_category_files(category)?;
    let mut out = String::new();
    out.push_str("## ");
    out.push_str(title);
    out.push_str(" (Auto)\n\n");
    render_category(&mut out, &files, include_samples)?;
    Ok(out)
}

fn render_category(
    out: &mut String,
    files: &[String],
    include_samples: bool,
) -> Result<(), String> {
    out.push_str("\n\n");
    if files.is_empty() {
        out.push_str("- (none)\n\n");
        return Ok(());
    }

    out.push_str("#### Index\n\n");
    for f in files {
        out.push_str("- `examples/");
        out.push_str(f);
        out.push_str("`\n");
    }
    if !include_samples {
        out.push('\n');
        return Ok(());
    }
    out.push('\n');
    out.push_str("#### Samples\n\n");
    for f in files {
        let path = format!("examples/{}", f);
        let body =
            fs::read_to_string(&path).map_err(|e| format!("failed to read {}: {}", path, e))?;
        out.push_str("##### `examples/");
        out.push_str(f);
        out.push_str("`\n\n");
        out.push_str("````loom\n");
        out.push_str(body.trim_end());
        out.push('\n');
        out.push_str("````\n\n");
    }
    Ok(())
}

fn list_category_files(category: &str) -> Result<Vec<String>, String> {
    let mut names = list_loom_files_recursive("examples")?;
    names.retain(|n| n.starts_with(&format!("{}/", category)));
    names.sort();
    Ok(names)
}
