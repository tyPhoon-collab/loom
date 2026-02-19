use loom::dsl::formatter::{self, FormattingMode};
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

        // Test Minimize Mode
        let expected_minimize_path = expected_dir.join("minimize").join(file_name);
        if expected_minimize_path.exists() {
            let expected = fs::read_to_string(&expected_minimize_path).unwrap();
            let actual =
                formatter::format_string_with_mode(&input_content, FormattingMode::Minimize);
            if actual.trim() != expected.trim() {
                failures.push(format!(
                    "Minimize mode mismatch for {}:\nExpected:\n---\n{}\n---\nActual:\n---\n{}\n---",
                    file_name, expected, actual
                ));
            }
        }

        // Test Justify Mode
        let expected_justify_path = expected_dir.join("justify").join(file_name);
        if expected_justify_path.exists() {
            let expected = fs::read_to_string(&expected_justify_path).unwrap();
            let actual =
                formatter::format_string_with_mode(&input_content, FormattingMode::Justify);
            if actual.trim() != expected.trim() {
                failures.push(format!(
                    "Justify mode mismatch for {}:\nExpected:\n---\n{}\n---\nActual:\n---\n{}\n---",
                    file_name, expected, actual
                ));
            }
        }

        // Test Equal Mode
        let expected_equal_path = expected_dir.join("equal").join(file_name);
        if expected_equal_path.exists() {
            let expected = fs::read_to_string(&expected_equal_path).unwrap();
            let actual = formatter::format_string_with_mode(&input_content, FormattingMode::Equal);
            if actual.trim() != expected.trim() {
                failures.push(format!(
                    "Equal mode mismatch for {}:\nExpected:\n---\n{}\n---\nActual:\n---\n{}\n---",
                    file_name, expected, actual
                ));
            }
        }

        // Test Time Mode
        let expected_time_path = expected_dir.join("time").join(file_name);
        if expected_time_path.exists() {
            let expected = fs::read_to_string(&expected_time_path).unwrap();
            let actual = formatter::format_string_with_mode(&input_content, FormattingMode::Time);
            if actual.trim() != expected.trim() {
                failures.push(format!(
                    "Time mode mismatch for {}:\nExpected:\n---\n{}\n---\nActual:\n---\n{}\n---",
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
