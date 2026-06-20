use loom::compiler::{Compiler, MidiEvent};
use loom::dsl::parser;
use std::fs;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "loom-template-libraries-{}-{}",
        name,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("libraries")).unwrap();
    fs::create_dir_all(dir.join("sections")).unwrap();
    dir
}

fn compiled_notes(song: &loom::dsl::token::Song) -> Vec<u8> {
    Compiler::new(song)
        .unwrap()
        .compile(song)
        .unwrap()
        .into_iter()
        .filter_map(|event| match event {
            MidiEvent::Note { note, .. } => Some(note),
            _ => None,
        })
        .collect()
}

#[test]
fn song_can_call_template_library_by_alias() {
    let dir = temp_dir("song");
    fs::write(
        dir.join("song.loom"),
        r#"---
templates:
  lib: libraries/lib.loom
---

# Lead: 1
[@lib.riff]
"#,
    )
    .unwrap();
    fs::write(dir.join("libraries/lib.loom"), "# @riff\nC4 | ^ |\n").unwrap();

    let song = parser::parse_song_from_path(&dir.join("song.loom")).unwrap();

    assert_eq!(compiled_notes(&song), vec![72]);
}

#[test]
fn fragment_can_call_manifest_template_library() {
    let dir = temp_dir("fragment");
    fs::write(
        dir.join("song.loom"),
        r#"---
fragments:
  intro: sections/intro.loom
templates:
  lib: libraries/lib.loom
---

# Lead: 1

[[intro]]
"#,
    )
    .unwrap();
    fs::write(dir.join("sections/intro.loom"), "# 1\n[@lib.riff]\n").unwrap();
    fs::write(dir.join("libraries/lib.loom"), "# @riff\nE4 | ^ |\n").unwrap();

    let song = parser::parse_song_from_path(&dir.join("song.loom")).unwrap();

    assert_eq!(compiled_notes(&song), vec![76]);
}

#[test]
fn template_library_can_call_nested_template_library() {
    let dir = temp_dir("nested");
    fs::write(
        dir.join("song.loom"),
        r#"---
templates:
  lib: libraries/lib.loom
---

# Lead: 1
[@lib.riff]
"#,
    )
    .unwrap();
    fs::write(
        dir.join("libraries/lib.loom"),
        r#"---
templates:
  common: common.loom
---

# @riff
[@common.hit]
"#,
    )
    .unwrap();
    fs::write(dir.join("libraries/common.loom"), "# @hit\nG4 | ^ |\n").unwrap();

    let song = parser::parse_song_from_path(&dir.join("song.loom")).unwrap();

    assert_eq!(compiled_notes(&song), vec![79]);
}

#[test]
fn local_template_call_does_not_resolve_to_library_alias() {
    let dir = temp_dir("local-only");
    fs::write(
        dir.join("song.loom"),
        r#"---
templates:
  riff: libraries/lib.loom
---

# Lead: 1
[@riff]
"#,
    )
    .unwrap();
    fs::write(dir.join("libraries/lib.loom"), "# @riff\nC4 | ^ |\n").unwrap();

    let song = parser::parse_song_from_path(&dir.join("song.loom")).unwrap();
    let err = Compiler::new(&song).unwrap().compile(&song).unwrap_err();
    let message = format!("{:?}", err);

    assert!(message.contains("Template not found"));
    assert!(message.contains("riff"));
}

#[test]
fn template_library_paths_follow_fragment_path_rules() {
    let dir = temp_dir("parent-path");
    fs::write(
        dir.join("song.loom"),
        r#"---
templates:
  lib: ../lib.loom
---

# Lead: 1
[@lib.riff]
"#,
    )
    .unwrap();

    let err = parser::parse_song_from_path(&dir.join("song.loom")).unwrap_err();

    assert!(err
        .to_string()
        .contains("Template library paths must not contain parent traversal"));
}

#[test]
fn circular_template_library_reference_is_rejected_while_loading() {
    let dir = temp_dir("cycle");
    fs::write(
        dir.join("song.loom"),
        r#"---
templates:
  a: libraries/a.loom
---

# Lead: 1
[@a.riff]
"#,
    )
    .unwrap();
    fs::write(
        dir.join("libraries/a.loom"),
        r#"---
templates:
  b: b.loom
---

# @riff
[@b.riff]
"#,
    )
    .unwrap();
    fs::write(
        dir.join("libraries/b.loom"),
        r#"---
templates:
  a: a.loom
---

# @riff
[@a.riff]
"#,
    )
    .unwrap();

    let err = parser::parse_song_from_path(&dir.join("song.loom")).unwrap_err();

    assert!(err
        .to_string()
        .contains("Circular template library reference detected"));
}
