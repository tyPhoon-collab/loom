use loom::dsl::formatter;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[test]
fn test_fmt_golden() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let fixtures_dir = Path::new(&manifest_dir).join("tests/fixtures/formatter");
    let input_dir = fixtures_dir.join("input");
    let expected_dir = fixtures_dir.join("expected");

    let mut failures = Vec::new();

    for entry in WalkDir::new(&input_dir)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("loom") {
            continue;
        }

        let file_name = path.file_name().unwrap().to_str().unwrap();
        let input_content = fs::read_to_string(path).unwrap();

        let expected_path = expected_dir.join(file_name);
        if expected_path.exists() {
            let actual = formatter::format_string(&input_content);
            let expected = fs::read_to_string(&expected_path).unwrap();
            if actual.trim() != expected.trim() {
                println!(
                    "MISMATCH in {}:\nACTUAL:\n---\n{}\n---\nEXPECTED:\n---\n{}\n---\n",
                    file_name, actual, expected
                );
                failures.push(format!(
                    "Mismatch for {}:\nExpected:\n---\n{}\n---\nActual:\n---\n{}\n---",
                    file_name, expected, actual
                ));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "Formatter Golden Tests Failed:\n\n{}",
            failures.join("\n\n")
        );
    }
}
