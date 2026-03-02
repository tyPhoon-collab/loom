use loom::compiler::compile_track_init_events;
use loom::compiler::Compiler;
use loom::compiler::MidiEvent;
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
    let events = compile_track_init_events(&song);

    assert_eq!(
        events,
        vec![
            MidiEvent::ControlChange {
                time: 0.0,
                channel: 1,
                cc: 0,
                value: 1
            },
            MidiEvent::ControlChange {
                time: 0.0,
                channel: 1,
                cc: 32,
                value: 2
            },
            MidiEvent::ProgramChange {
                time: 0.0,
                channel: 1,
                program: 40
            },
            MidiEvent::ControlChange {
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

#[test]
fn test_pan_macro_emits_timed_cc10() {
    let source = r#"
# Lead: 1
[@a|pan:16][@a|pan:100]

# @a
C4 | ^ ^ |
"#;
    let song = parse_song(source.to_string()).unwrap();
    let compiler = Compiler::new(&song).unwrap();
    let controls = compiler.compile(&song).unwrap();

    let pan_events: Vec<_> = controls
        .into_iter()
        .filter_map(|e| match e {
            MidiEvent::ControlChange {
                time,
                channel,
                cc,
                value,
            } if cc == 10 => Some((time, channel, value)),
            _ => None,
        })
        .collect();

    assert_eq!(pan_events, vec![(0.0, 0, 16), (4.0, 0, 100)]);
}
