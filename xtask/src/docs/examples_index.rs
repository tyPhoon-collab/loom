use std::fs;
use std::path::{Path, PathBuf};

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
        render_related_fragments(out, Path::new(&path))?;
        render_related_libraries(out, Path::new(&path))?;
    }
    Ok(())
}

fn list_category_files(category: &str) -> Result<Vec<String>, String> {
    let mut names = list_loom_files_recursive("examples")?;
    names.retain(|n| n.starts_with(&format!("{}/", category)));
    names.retain(|n| {
        !n.split('/')
            .any(|part| matches!(part, "sections" | "libraries"))
    });
    names.sort();
    Ok(names)
}

fn render_related_fragments(out: &mut String, manifest_path: &Path) -> Result<(), String> {
    let Some(parent) = manifest_path.parent() else {
        return Ok(());
    };
    let sections_dir = parent.join("sections");
    if !sections_dir.is_dir() {
        return Ok(());
    }

    let mut fragments = Vec::new();
    collect_section_fragments(&sections_dir, &mut fragments)?;
    fragments.sort();
    if fragments.is_empty() {
        return Ok(());
    }

    out.push_str("Related fragments:\n\n");
    for fragment in fragments {
        let body = fs::read_to_string(&fragment)
            .map_err(|e| format!("failed to read {}: {}", fragment.display(), e))?;
        out.push_str("````loom\n");
        out.push_str("> ");
        out.push_str(&fragment.to_string_lossy().replace('\\', "/"));
        out.push('\n');
        out.push_str(body.trim_end());
        out.push('\n');
        out.push_str("````\n\n");
    }
    Ok(())
}

fn render_related_libraries(out: &mut String, song_path: &Path) -> Result<(), String> {
    let Some(parent) = song_path.parent() else {
        return Ok(());
    };
    let libraries_dir = parent.join("libraries");
    if !libraries_dir.is_dir() {
        return Ok(());
    }

    let mut libraries = Vec::new();
    collect_libraries(&libraries_dir, &mut libraries)?;
    libraries.sort();
    if libraries.is_empty() {
        return Ok(());
    }

    out.push_str("Related template libraries:\n\n");
    for library in libraries {
        let body = fs::read_to_string(&library)
            .map_err(|e| format!("failed to read {}: {}", library.display(), e))?;
        out.push_str("````loom\n");
        out.push_str("> ");
        out.push_str(&library.to_string_lossy().replace('\\', "/"));
        out.push('\n');
        out.push_str(body.trim_end());
        out.push('\n');
        out.push_str("````\n\n");
    }
    Ok(())
}

fn collect_libraries(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(dir).map_err(|e| format!("failed to read {}: {}", dir.display(), e))?
    {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_libraries(&path, out)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("loom") {
            out.push(path);
        }
    }
    Ok(())
}

fn collect_section_fragments(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries =
        fs::read_dir(dir).map_err(|e| format!("failed to read {}: {}", dir.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {}", e))?;
        let path = entry.path();
        if path.is_dir() {
            collect_section_fragments(&path, out)?;
            continue;
        }
        if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext == "loom")
        {
            out.push(path);
        }
    }
    Ok(())
}
