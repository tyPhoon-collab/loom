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
snare | . . |
kick  | ^   |
"#;

    let formatted = formatter::format_string(input);

    assert_eq!(formatted, expected, "Formatted output mismatch");
}

#[test]
fn test_fmt_sort_pitch() {
    let input = r#"
c3 | ^ |
e3 | ^ |
g3 | ^ |
"#;

    // Expected: High -> Low (G3, E3, C3)
    let expected = r#"
g3 | ^ |
e3 | ^ |
c3 | ^ |
"#;
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, expected);
}

#[test]
fn test_fmt_sort_drums() {
    let input = r#"
kick  | ^ |
snare | ^ |
hihat | ^ |
"#;

    // MIDI Numbers:
    // Kick (36)
    // Snare (38)
    // Hihat (42)
    //
    // Sorted High -> Low: Hihat, Snare, Kick
    let expected = r#"
hihat | ^ |
snare | ^ |
kick  | ^ |
"#;
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, expected);
}

#[test]
fn test_preserve_comments() {
    let input = "> comment\n# Track: 1\n";
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, "> comment\n# Track: 1\n");
}
