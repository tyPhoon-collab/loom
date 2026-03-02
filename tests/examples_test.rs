use loom::dsl::parser;
use loom::inspect::{collect_track_events, TrackEvent};
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
            let mut events = collect_track_events(&song)
                .unwrap_or_else(|_| panic!("Failed to compile example: {}", filename));
            sort_track_events_for_snapshot(&mut events);

            // Snapshot test
            insta::with_settings!({
                snapshot_path => "snapshots",
                prepend_module_to_snapshot => false,
            }, {
                insta::assert_debug_snapshot!(filename, events);
            });
        }
    }
}

fn sort_track_events_for_snapshot(events: &mut [TrackEvent]) {
    events.sort_by(|a, b| {
        a.event
            .time()
            .partial_cmp(&b.event.time())
            .unwrap()
            .then_with(|| a.track.cmp(&b.track))
            .then_with(|| a.event.channel().cmp(&b.event.channel()))
            .then_with(|| a.event.timing_order().cmp(&b.event.timing_order()))
            .then_with(|| {
                a.event
                    .note()
                    .unwrap_or(3)
                    .cmp(&b.event.note().unwrap_or(3))
            })
            .then_with(|| {
                a.event
                    .velocity()
                    .unwrap_or(0)
                    .cmp(&b.event.velocity().unwrap_or(0))
            })
            .then_with(|| a.event.cc().unwrap_or(0).cmp(&b.event.cc().unwrap_or(0)))
            .then_with(|| {
                a.event
                    .value()
                    .unwrap_or(0)
                    .cmp(&b.event.value().unwrap_or(0))
            })
            .then_with(|| {
                a.event
                    .program()
                    .unwrap_or(0)
                    .cmp(&b.event.program().unwrap_or(0))
            })
    });
}
