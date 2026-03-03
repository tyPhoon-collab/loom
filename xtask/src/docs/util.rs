use std::fs;
use std::path::{Path, PathBuf};

pub fn list_loom_files_flat(dir: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| format!("failed to read {}: {}", dir, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {}", e))?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|ext| ext == "loom")
        {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                out.push(name.to_string());
            }
        }
    }
    Ok(out)
}

pub fn list_loom_files_recursive(dir: &str) -> Result<Vec<String>, String> {
    let root = Path::new(dir);
    let mut out = Vec::new();
    walk_loom_files(root, root, &mut out)?;
    Ok(out)
}

fn walk_loom_files(root: &Path, current: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = fs::read_dir(current)
        .map_err(|e| format!("failed to read {}: {}", current.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("failed to read dir entry: {}", e))?;
        let path: PathBuf = entry.path();
        if path.is_dir() {
            walk_loom_files(root, &path, out)?;
            continue;
        }
        if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext == "loom")
        {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| format!("failed to strip prefix: {}", e))?
                .to_string_lossy()
                .replace('\\', "/");
            out.push(rel);
        }
    }
    Ok(())
}

pub fn replace_between_markers(
    content: &str,
    start_marker: &str,
    end_marker: &str,
    generated: &str,
) -> Result<String, String> {
    let start = content
        .find(start_marker)
        .ok_or_else(|| format!("missing start marker: {}", start_marker))?;
    let end = content
        .find(end_marker)
        .ok_or_else(|| format!("missing end marker: {}", end_marker))?;
    if end < start {
        return Err("marker order is invalid".to_string());
    }

    let mut out = String::new();
    out.push_str(&content[..start + start_marker.len()]);
    out.push('\n');
    out.push('\n');
    out.push_str(generated.trim_end());
    out.push('\n');
    out.push('\n');
    out.push_str(&content[end..]);
    Ok(out)
}
