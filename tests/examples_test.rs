use loom::dsl::parser;
use loom::inspect::{collect_track_events, TrackEvent};
use std::path::Path;
use walkdir::WalkDir;

#[test]
fn test_examples() {
    let examples_dir = Path::new("examples");
    let mut files = Vec::new();
    for entry in WalkDir::new(examples_dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("loom") {
            continue;
        }
        let rel = path.strip_prefix(examples_dir).expect("strip examples/");
        if rel.starts_with("internals") {
            continue;
        }
        if rel.components().any(|component| {
            matches!(
                component.as_os_str().to_str(),
                Some("sections" | "libraries")
            )
        }) {
            continue;
        }
        files.push(path.to_path_buf());
    }
    files.sort();

    for path in files {
        let filename = path.file_name().unwrap().to_str().unwrap();
        let rel = path.strip_prefix(examples_dir).expect("strip examples/");
        let snapshot_name = if filename == "song.loom" {
            rel.to_string_lossy().replace('/', "__")
        } else {
            filename.to_string()
        };

        println!("Testing example: {}", rel.display());

        let song = parser::parse_song_from_path(&path)
            .unwrap_or_else(|_| panic!("Failed to parse example: {}", rel.display()));
        let mut events = collect_track_events(&song)
            .unwrap_or_else(|_| panic!("Failed to compile example: {}", rel.display()));
        sort_track_events_for_snapshot(&mut events);

        // Snapshot test
        insta::with_settings!({
            snapshot_path => "snapshots",
            prepend_module_to_snapshot => false,
        }, {
            insta::assert_debug_snapshot!(snapshot_name, events);
        });
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
