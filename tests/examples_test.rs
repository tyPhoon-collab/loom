use loom::compiler;
use loom::compiler::MidiInitEvent;
use loom::dsl::parser;
use std::fs;
use std::path::Path;

#[test]
fn test_examples() {
    let examples_dir = Path::new("examples");
    let entries = fs::read_dir(examples_dir).expect("Failed to read examples directory");

    for entry in entries {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("loom") {
            let filename = path.file_name().unwrap().to_str().unwrap();
            let content = fs::read_to_string(&path).expect("Failed to read example file");

            println!("Testing example: {}", filename);

            let song = parser::parse_song(content)
                .unwrap_or_else(|_| panic!("Failed to parse example: {}", filename));
            let compiler = compiler::Compiler::new(&song).expect("Failed to create compiler");
            let (events, mut control_events) = compiler
                .compile_with_controls(&song)
                .unwrap_or_else(|_| panic!("Failed to compile example: {}", filename));

            // Snapshot test
            insta::with_settings!({
                snapshot_path => "snapshots",
                prepend_module_to_snapshot => false,
            }, {
                insta::assert_debug_snapshot!(filename, events);
                if !control_events.is_empty() {
                    sort_controls_for_snapshot(&mut control_events);
                    let controls_snapshot = format!("{}.controls", filename);
                    insta::assert_debug_snapshot!(controls_snapshot, control_events);
                }
            });
        }
    }
}

fn sort_controls_for_snapshot(events: &mut [MidiInitEvent]) {
    events.sort_by(|a, b| {
        control_time(a)
            .partial_cmp(&control_time(b))
            .unwrap()
            .then_with(|| control_order(a).cmp(&control_order(b)))
            .then_with(|| control_channel(a).cmp(&control_channel(b)))
            .then_with(|| control_data(a).cmp(&control_data(b)))
    });
}

fn control_time(event: &MidiInitEvent) -> f64 {
    match event {
        MidiInitEvent::ControlChange { time, .. } | MidiInitEvent::ProgramChange { time, .. } => {
            *time
        }
    }
}

fn control_order(event: &MidiInitEvent) -> u8 {
    match event {
        MidiInitEvent::ControlChange { cc: 0, .. } => 0,
        MidiInitEvent::ControlChange { cc: 32, .. } => 1,
        MidiInitEvent::ProgramChange { .. } => 2,
        MidiInitEvent::ControlChange { .. } => 3,
    }
}

fn control_channel(event: &MidiInitEvent) -> u8 {
    match event {
        MidiInitEvent::ControlChange { channel, .. }
        | MidiInitEvent::ProgramChange { channel, .. } => *channel,
    }
}

fn control_data(event: &MidiInitEvent) -> (u8, u8) {
    match event {
        MidiInitEvent::ControlChange { cc, value, .. } => (*cc, *value),
        MidiInitEvent::ProgramChange { program, .. } => (255, *program),
    }
}
