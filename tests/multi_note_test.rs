use loom::compiler::Compiler;
use loom::parser::parse_song;

#[test]
fn test_compile_multi_note() {
    let source = r#"---
bpm: 120
---
# Track: 1
c3,e3 | ^ |
"#;
    let song = parse_song(source.to_string()).unwrap();
    let compiler = Compiler::new(&song);
    let events = compiler.compile(&song).unwrap();

    // C3=60, E3=64
    assert_eq!(events.len(), 2);
    assert!(events.iter().any(|e| e.note == 60));
    assert!(events.iter().any(|e| e.note == 64));
    assert_eq!(events[0].time, 0.0);
    assert_eq!(events[1].time, 0.0);
}

#[test]
fn test_multi_note_sustain() {
    let source = r#"---
bpm: 120
---
# Track: 1
c3,e3 | ^ - |
"#;
    let song = parse_song(source.to_string()).unwrap();
    let compiler = Compiler::new(&song);
    let events = compiler.compile(&song).unwrap();

    assert_eq!(events.len(), 2);
    // duration: in 4/4, 1 bar (unit=bar) is 4.0 beats.
    // 1 block = 4.0. 2 tokens (^, -) mean duration_per_token = 4.0 / 2 = 2.0.
    // ^ (2.0) + - (2.0) = 4.0 total.
    assert_eq!(events[0].duration, 4.0);
    assert_eq!(events[1].duration, 4.0);
}
