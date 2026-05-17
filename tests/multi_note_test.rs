use loom::compiler::CompileError;
use loom::compiler::Compiler;
use loom::compiler::MidiEvent;
use loom::dsl::parser::parse_song;

fn has_variant(err: &CompileError, predicate: &dyn Fn(&CompileError) -> bool) -> bool {
    if predicate(err) {
        return true;
    }
    match err {
        CompileError::Context { source, .. } => has_variant(source, predicate),
        _ => false,
    }
}

#[test]
fn test_compile_multi_note() {
    let source = r#"---
bpm: 120
---
# Track: 1
c3,e3 | ^ |
"#;
    let song = parse_song(source.to_string()).unwrap();
    let compiler = Compiler::new(&song).unwrap();
    let events = compiler.compile(&song).unwrap();
    let note_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, MidiEvent::Note { .. }))
        .collect();

    // C3=60, E3=64
    assert_eq!(note_events.len(), 2);
    assert!(note_events
        .iter()
        .any(|e| matches!(e, MidiEvent::Note { note: 60, .. })));
    assert!(note_events
        .iter()
        .any(|e| matches!(e, MidiEvent::Note { note: 64, .. })));
    assert!(note_events
        .iter()
        .all(|e| matches!(e, MidiEvent::Note { time: 0.0, .. })));
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
    let compiler = Compiler::new(&song).unwrap();
    let events = compiler.compile(&song).unwrap();
    let note_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, MidiEvent::Note { .. }))
        .collect();

    assert_eq!(note_events.len(), 2);
    // duration: in 4/4, 1 bar (unit=bar) is 4.0 beats.
    // 1 block = 4.0. 2 tokens (^, -) mean duration_per_token = 4.0 / 2 = 2.0.
    // ^ (2.0) + - (2.0) = 4.0 total.
    assert!(note_events
        .iter()
        .all(|e| matches!(e, MidiEvent::Note { duration: 4.0, .. })));
}

#[test]
fn test_modifier_latch_applies_on_later_non_empty_block() {
    let source = r#"---
bpm: 152
---
# Piano: 2
F4,C5 |              | ^   ^ ^ ^ [^ ^ [^ ^ ^]]  |
v     |              | !60                      |
G4,B4 | ^   ^ ^ ^ ^  |                          |
v     | !60          |                          |
"#;

    let song = parse_song(source.to_string()).unwrap();
    let compiler = Compiler::new(&song).unwrap();
    let events = compiler.compile(&song).unwrap();
    let note_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, MidiEvent::Note { .. }))
        .collect();

    // Regression: modifier blocks must stay aligned even if pattern block has zero tokens.
    assert!(!note_events.is_empty());
    assert!(note_events
        .iter()
        .all(|e| matches!(e, MidiEvent::Note { velocity: 60, .. })));
}

#[test]
fn test_velocity_out_of_range_is_compile_error() {
    let source = r#"
# Track: 1
C4 | ^ |
v  | 200 |
"#;
    let song = parse_song(source.to_string()).unwrap();
    let compiler = Compiler::new(&song).unwrap();
    let err = compiler.compile(&song).unwrap_err();
    let compile_err = err
        .downcast_ref::<CompileError>()
        .expect("error should be CompileError");
    assert!(has_variant(compile_err, &|e| {
        matches!(e, CompileError::VelocityOutOfRange { .. })
    }));
}

#[test]
fn test_pitch_result_out_of_range_is_compile_error() {
    let source = r#"
# Track: 1
C4 | ^ |
p  | +200 |
"#;
    let song = parse_song(source.to_string()).unwrap();
    let compiler = Compiler::new(&song).unwrap();
    let err = compiler.compile(&song).unwrap_err();
    let compile_err = err
        .downcast_ref::<CompileError>()
        .expect("error should be CompileError");
    assert!(has_variant(compile_err, &|e| {
        matches!(e, CompileError::NoteOutOfRange { .. })
    }));
}
