use loom::dsl::parser::parse_song;
use loom::inspect::collect_track_events;

#[test]
fn test_collect_track_events_includes_note_and_controls() {
    let source = r#"
# Lead: 1
## bank 0/32
## sound 40
## pan 64
C4 | ^ |
"#;
    let song = parse_song(source.to_string()).unwrap();
    let events = collect_track_events(&song).unwrap();

    assert!(events
        .iter()
        .any(|e| matches!(e.event, loom::compiler::MidiEvent::Note { note: 72, .. })));
    assert!(events.iter().any(|e| matches!(
        e.event,
        loom::compiler::MidiEvent::ControlChange {
            cc: 0,
            value: 0,
            ..
        }
    )));
    assert!(events.iter().any(|e| matches!(
        e.event,
        loom::compiler::MidiEvent::ControlChange {
            cc: 32,
            value: 32,
            ..
        }
    )));
    assert!(events.iter().any(|e| matches!(
        e.event,
        loom::compiler::MidiEvent::ProgramChange { program: 40, .. }
    )));
    assert!(events.iter().any(|e| matches!(
        e.event,
        loom::compiler::MidiEvent::ControlChange {
            cc: 10,
            value: 64,
            ..
        }
    )));
}
