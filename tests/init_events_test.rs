use loom::compiler::collect_init_events;
use loom::compiler::MidiInitEvent;
use loom::dsl::parser::parse_song;
use loom::dsl::token::TrackInitEvent;

#[test]
fn test_parse_track_init_events_with_sugar() {
    let source = r#"
# Piano: 2
## bank 0/32
## pc 40
## pan 64
## volume 100
C4 | ^ |
"#;
    let song = parse_song(source.to_string()).unwrap();
    let track = &song.tracks[0];

    assert_eq!(
        track.init_events,
        vec![
            TrackInitEvent::BankSelect { msb: 0, lsb: 32 },
            TrackInitEvent::ProgramChange { program: 40 },
            TrackInitEvent::ControlChange { cc: 10, value: 64 },
            TrackInitEvent::ControlChange { cc: 7, value: 100 },
        ]
    );
}

#[test]
fn test_collect_init_events_order() {
    let source = r#"
# Piano: 2
## bank 1/2
## pc 40
## cc 11 90
C4 | ^ |
"#;
    let song = parse_song(source.to_string()).unwrap();
    let events = collect_init_events(&song);

    assert_eq!(
        events,
        vec![
            MidiInitEvent::ControlChange {
                time: 0.0,
                channel: 1,
                cc: 0,
                value: 1
            },
            MidiInitEvent::ControlChange {
                time: 0.0,
                channel: 1,
                cc: 32,
                value: 2
            },
            MidiInitEvent::ProgramChange {
                time: 0.0,
                channel: 1,
                program: 40
            },
            MidiInitEvent::ControlChange {
                time: 0.0,
                channel: 1,
                cc: 11,
                value: 90
            },
        ]
    );
}

#[test]
fn test_duplicate_pc_is_error() {
    let source = r#"
# Piano: 1
## pc 1
## pc 2
C4 | ^ |
"#;
    let err = parse_song(source.to_string()).unwrap_err().to_string();
    assert!(err.contains("Duplicate program change"));
}

#[test]
fn test_sound_is_alias_of_pc_and_duplicate_is_error() {
    let source = r#"
# Piano: 1
## sound 40
## pc 41
C4 | ^ |
"#;
    let err = parse_song(source.to_string()).unwrap_err().to_string();
    assert!(err.contains("Duplicate program change"));
}

#[test]
fn test_bank_and_cc0_conflict_is_error() {
    let source = r#"
# Piano: 1
## bank 0/1
## cc 0 10
C4 | ^ |
"#;
    let err = parse_song(source.to_string()).unwrap_err().to_string();
    assert!(err.contains("Cannot mix"));
}
