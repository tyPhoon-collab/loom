use loom::formatter;

#[test]
fn test_basic_vertical_alignment() {
    let input = r#"
# Drums: 1
kick|^|
snare|. .|
"#;

    let expected = r#"
# Drums: 1
kick  | ^   |
snare | . . |
"#;

    let formatted = formatter::format_string(input);

    assert_eq!(formatted, expected, "Formatted output mismatch");
}

#[test]
fn test_preserve_comments() {
    let input = "> comment\n# Track: 1\n";
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, "> comment\n# Track: 1\n");
}
