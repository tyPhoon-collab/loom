use loom::formatter;

#[test]
fn test_fmt_irregular_spacing() {
    let input = r#"
# Track: 1
kick| ^ .|^ .|
snare  |  . . | . ^|
"#;

    let expected = r#"
# Track: 1
kick  | ^ . | ^ . |
snare | . . | . ^ |
"#;
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, expected);
}

#[test]
fn test_fmt_empty_blocks() {
    let input = r#"
# Track: 1
kick | | ^ |
snare | ^ | |
"#;

    let expected = r#"
# Track: 1
kick  |   | ^ |
snare | ^ |   |
"#;
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, expected);
}

#[test]
fn test_fmt_groups() {
    let input = r#"
# Track: 1
kick  | ^ |
snare | [^ ^ ^] |
"#;

    // kick  | ^       |
    // snare | [^ ^ ^] |
    // ^ has width 1. [^ ^ ^] has width 7.
    // max width 7.
    // kick gets padded.

    let expected = r#"
# Track: 1
kick  | ^       |
snare | [^ ^ ^] |
"#;
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, expected);
}

#[test]
fn test_mixed_block_counts() {
    let input = r#"
# Track: 1
short | ^ |
long  | ^ | ^ |
"#;

    let expected = r#"
# Track: 1
short | ^ |
long  | ^ | ^ |
"#;
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, expected);
}

#[test]
fn test_trailing_comment_alignment() {
    // Comments are preserved but not necessarily aligned with each other (current logic only aligns pattern parts)
    let input = r#"
# Track: 1
k | ^ | > comment
s | ^ | > another
"#;
    let expected = r#"
# Track: 1
k | ^ | > comment
s | ^ | > another
"#;
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, expected);
}
