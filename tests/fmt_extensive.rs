use loom::dsl::formatter;

#[test]
fn test_fmt_irregular_spacing() {
    let input = r#"
# Track: 1
kick| ^ .|^ .|
snare  |  . . | . ^|
"#;

    let expected = r#"
# Track: 1
snare | . . | . ^ |
kick  | ^ . | ^ . |
"#;
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, expected);
}

#[test]
fn test_fmt_empty_blocks() {
    let input = r#"
# Track: 1
E4 | | ^ |
C4 | ^ | |
"#;

    let expected = r#"
# Track: 1
E4 | | ^ |
C4 | ^ | |
"#;
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, expected);
}

#[test]
fn test_fmt_groups() {
    let input = r#"
# Track: 1
C4  | ^ |
E4 | [^ ^ ^] |
"#;

    let expected = r#"
# Track: 1
E4 | [^ ^ ^] |
C4 | ^       |
"#;
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, expected);
}

#[test]
fn test_mixed_block_counts() {
    let input = r#"
# Track: 1
C4 | ^ |
D4  | ^ | ^ |
"#;

    let expected = r#"
# Track: 1
D4 | ^ | ^ |
C4 | ^ |
"#;
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, expected);
}

#[test]
fn test_trailing_comment_alignment() {
    // Comments are preserved but not necessarily aligned with each other (current logic only aligns pattern parts)
    let input = r#"
# Track: 1
C4 | ^ | > comment
D4 | ^ | > another
"#;
    let expected = r#"
# Track: 1
D4 | ^ | > another
C4 | ^ | > comment
"#;
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, expected);
}

#[test]
fn test_fmt_alignment_mixed_blocks() {
    let input = r#"
G3 | . . ^ |
E3 | . ^ . |
C3 | ^ . . |
C2 | ^ |
"#;

    let expected = r#"
G3 | . . ^ |
E3 | . ^ . |
C3 | ^ . . |
C2 | ^ |
"#;
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, expected);
}

#[test]
fn test_fmt_empty_and_displaced_blocks() {
    let input = r#"
F3 ||^ - -|
C3 | ^ - - | ..^ |
E3 | . ^ . | . ^ . |
G3 | . . ^ | |
"#;

    let expected = r#"
G3 | . . ^ | |
F3 | | ^ - - |
E3 | . ^ . | . ^ . |
C3 | ^ - - | . . ^ |
"#;
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, expected);
}

#[test]
fn test_fmt_dense_tokens() {
    let input = r#"
# Track: 1
C3 | ^.. |
E3 | .^. |
G3 | ..^ |
"#;

    let expected = r#"
# Track: 1
G3 | . . ^ |
E3 | . ^ . |
C3 | ^ . . |
"#;
    let formatted = formatter::format_string(input);
    assert_eq!(formatted, expected);
}
