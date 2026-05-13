use loom::compiler::{Compiler, MidiEvent};
use loom::dsl::parser;

fn compile(input: &str) -> Vec<MidiEvent> {
    let song = parser::parse_song(input.to_string()).unwrap();
    let compiler = Compiler::new(&song).unwrap();
    compiler.compile(&song).unwrap()
}

fn note_events(events: &[MidiEvent]) -> Vec<(f64, u8)> {
    events
        .iter()
        .filter_map(|event| match event {
            MidiEvent::Note { time, velocity, .. } => Some((*time, *velocity)),
            _ => None,
        })
        .collect()
}

#[test]
fn humanize_is_off_by_default() {
    let input = r#"
# Piano: 1
C4 | ^ . ^ . |
"#;

    let events = compile(input);
    assert_eq!(note_events(&events), vec![(0.0, 100), (2.0, 100)]);
}

#[test]
fn humanize_true_applies_deterministic_note_variation() {
    let input = r#"---
humanize: true
---

# Piano: 1
C4 | ^ . ^ . |
"#;

    let first = compile(input);
    let second = compile(input);

    assert_eq!(first, second);
    let notes = note_events(&first);
    assert_eq!(notes.len(), 2);
    assert!(
        notes != vec![(0.0, 100), (2.0, 100)],
        "humanize should alter note timing or velocity"
    );
    assert!(notes
        .iter()
        .all(|(time, velocity)| *time >= 0.0 && (1..=127).contains(velocity)));
}

#[test]
fn humanize_config_can_disable_one_axis_and_preserves_controls() {
    let input = r#"---
humanize:
  timing: 0
  velocity: 5
  seed: 42
---

# Piano: 1
## bank 0/32
## pc 4
## cc 11 100
C4 | ^ . ^ . |
"#;

    let events = compile(input);
    let notes = note_events(&events);

    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0].0, 0.0);
    assert_eq!(notes[1].0, 2.0);
    assert!(notes.iter().any(|(_, velocity)| *velocity != 100));

    assert!(events.iter().any(|event| matches!(
        event,
        MidiEvent::ControlChange {
            time: 0.0,
            channel: 0,
            cc: 0,
            value: 0
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        MidiEvent::ControlChange {
            time: 0.0,
            channel: 0,
            cc: 32,
            value: 32
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        MidiEvent::ProgramChange {
            time: 0.0,
            channel: 0,
            program: 4
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        MidiEvent::ControlChange {
            time: 0.0,
            channel: 0,
            cc: 11,
            value: 100
        }
    )));

    let without_controls = compile(
        r#"---
humanize:
  timing: 0
  velocity: 5
  seed: 42
---

# Piano: 1
C4 | ^ . ^ . |
"#,
    );
    assert_eq!(notes, note_events(&without_controls));
}

#[test]
fn humanize_config_defaults_omitted_fields() {
    let input = r#"---
humanize:
  timing: 0
---

# Piano: 1
C4 | ^ . ^ . |
"#;

    let events = compile(input);
    let notes = note_events(&events);

    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0].0, 0.0);
    assert_eq!(notes[1].0, 2.0);
    assert!(
        notes.iter().any(|(_, velocity)| *velocity != 100),
        "omitted velocity should use the default humanize amount"
    );
}
