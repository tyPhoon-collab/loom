use loom::compiler::{Compiler, MidiEvent};
use loom::dsl::parser;
use std::fs;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "loom-manifest-fragments-{}-{}",
        name,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("sections")).unwrap();
    dir
}

#[test]
fn manifest_fragments_play_in_manifest_order_with_silent_missing_tracks() {
    let dir = temp_dir("order");
    fs::write(
        dir.join("song.loom"),
        r#"---
title: Demo
fragments:
  intro: sections/intro.loom
  chorus: sections/chorus.loom
---

# Piano: 1
# Bass: 2

[[intro]]
[[chorus]]
"#,
    )
    .unwrap();
    fs::write(dir.join("sections/intro.loom"), "# 1\nC4 | ^ |\n").unwrap();
    fs::write(dir.join("sections/chorus.loom"), "# 2\nC2 | ^ |\n").unwrap();

    let song = parser::parse_song_from_path(&dir.join("song.loom")).unwrap();
    let events = Compiler::new(&song).unwrap().compile(&song).unwrap();
    let bass_note_time = events
        .iter()
        .find_map(|event| match event {
            MidiEvent::Note {
                channel: 1, time, ..
            } => Some(*time),
            _ => None,
        })
        .unwrap();

    assert_eq!(bass_note_time, 4.0);
}

#[test]
fn fragment_templates_are_local_to_each_fragment() {
    let dir = temp_dir("templates");
    fs::write(
        dir.join("song.loom"),
        r#"---
fragments:
  a: sections/a.loom
  b: sections/b.loom
---

# Lead: 1

[[a]]
[[b]]
"#,
    )
    .unwrap();
    fs::write(
        dir.join("sections/a.loom"),
        "# @riff\nC4 | ^ |\n\n# 1\n[@riff]\n",
    )
    .unwrap();
    fs::write(
        dir.join("sections/b.loom"),
        "# @riff\nE4 | ^ |\n\n# 1\n[@riff]\n",
    )
    .unwrap();

    let song = parser::parse_song_from_path(&dir.join("song.loom")).unwrap();
    let events = Compiler::new(&song).unwrap().compile(&song).unwrap();
    let notes: Vec<u8> = events
        .iter()
        .filter_map(|event| match event {
            MidiEvent::Note { note, .. } => Some(*note),
            _ => None,
        })
        .collect();

    assert_eq!(notes, vec![72, 76]);
}

#[test]
fn fragment_rejects_duplicate_track_reference() {
    let dir = temp_dir("duplicate");
    fs::write(
        dir.join("song.loom"),
        r#"---
fragments:
  a: sections/a.loom
---

# Lead: 1

[[a]]
"#,
    )
    .unwrap();
    fs::write(
        dir.join("sections/a.loom"),
        "# 1\nC4 | ^ |\n\n# 1\nE4 | ^ |\n",
    )
    .unwrap();

    let err = parser::parse_song_from_path(&dir.join("song.loom")).unwrap_err();

    assert!(err.to_string().contains("Duplicate track reference"));
}

#[test]
fn manifest_solo_filters_fragment_tracks_even_when_solo_track_is_absent() {
    let dir = temp_dir("solo");
    fs::write(
        dir.join("song.loom"),
        r#"---
fragments:
  a: sections/a.loom
---

# Lead: 1 s
# Bass: 2

[[a]]
"#,
    )
    .unwrap();
    fs::write(dir.join("sections/a.loom"), "# 2\nC2 | ^ |\n").unwrap();

    let song = parser::parse_song_from_path(&dir.join("song.loom")).unwrap();
    let events = Compiler::new(&song).unwrap().compile(&song).unwrap();

    assert!(events
        .iter()
        .all(|event| !matches!(event, MidiEvent::Note { .. })));
}
