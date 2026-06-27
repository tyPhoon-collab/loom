use loom::compiler::MidiEvent;
use loom::playground::{
    compile_workspace, compile_workspace_with_diagnostics, PlaygroundCompileOutput,
    PlaygroundWorkspace,
};
use std::collections::HashMap;

fn workspace(entry_path: &str, active_path: &str, files: &[(&str, &str)]) -> PlaygroundWorkspace {
    PlaygroundWorkspace {
        entry_path: entry_path.to_string(),
        active_path: active_path.to_string(),
        files: files
            .iter()
            .map(|(path, source)| (path.to_string(), source.to_string()))
            .collect::<HashMap<_, _>>(),
    }
}

fn compiled_notes(workspace: PlaygroundWorkspace) -> Vec<u8> {
    compile_workspace(&workspace)
        .unwrap()
        .into_iter()
        .filter_map(|event| match event {
            MidiEvent::Note { note, .. } => Some(note),
            _ => None,
        })
        .collect()
}

#[test]
fn virtual_workspace_compiles_single_song() {
    let notes = compiled_notes(workspace(
        "song.loom",
        "song.loom",
        &[("song.loom", "# Lead: 1\nC4 | ^ |\n")],
    ));

    assert_eq!(notes, vec![72]);
}

#[test]
fn virtual_workspace_compiles_manifest_fragments() {
    let notes = compiled_notes(workspace(
        "song.loom",
        "sections/intro.loom",
        &[
            (
                "song.loom",
                r#"---
fragments:
  intro: sections/intro.loom
---

# Lead: 1

[[intro]]
"#,
            ),
            ("sections/intro.loom", "# 1\nE4 | ^ |\n"),
        ],
    ));

    assert_eq!(notes, vec![76]);
}

#[test]
fn virtual_workspace_compiles_manifest_template_library() {
    let notes = compiled_notes(workspace(
        "song.loom",
        "song.loom",
        &[
            (
                "song.loom",
                r#"---
templates:
  lib: libraries/lib.loom
---

# Lead: 1
[@lib.riff]
"#,
            ),
            ("libraries/lib.loom", "# @riff\nG4 | ^ |\n"),
        ],
    ));

    assert_eq!(notes, vec![79]);
}

#[test]
fn virtual_workspace_reports_missing_fragment_file() {
    let err = compile_workspace(&workspace(
        "song.loom",
        "song.loom",
        &[(
            "song.loom",
            r#"---
fragments:
  intro: sections/intro.loom
---

# Lead: 1

[[intro]]
"#,
        )],
    ))
    .unwrap_err();

    assert!(err.to_string().contains("Cannot read fragment"));
    assert!(err.to_string().contains("sections/intro.loom"));
}

#[test]
fn virtual_workspace_rejects_backslash_paths() {
    let err = compile_workspace(&workspace(
        "song.loom",
        "song.loom",
        &[("sections\\intro.loom", "# 1\nC4 | ^ |\n")],
    ))
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("Workspace paths must use `/` separators"));
}

#[test]
fn virtual_workspace_requires_active_path() {
    let err = compile_workspace(&workspace(
        "song.loom",
        "missing.loom",
        &[("song.loom", "# Lead: 1\nC4 | ^ |\n")],
    ))
    .unwrap_err();

    assert!(err.to_string().contains("Workspace active path"));
}

#[test]
fn virtual_workspace_diagnostics_include_entry_path_and_position() {
    let output = compile_workspace_with_diagnostics(&workspace(
        "song.loom",
        "song.loom",
        &[("song.loom", "# Lead: 1\nnot loom\n")],
    ));

    let PlaygroundCompileOutput::Err { diagnostics } = output else {
        panic!("expected diagnostics");
    };
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].path.as_deref(), Some("song.loom"));
    assert_eq!(diagnostics[0].line, Some(2));
    assert_eq!(diagnostics[0].column, Some(1));
    assert!(diagnostics[0].message.contains("Parse error"));
}

#[test]
fn virtual_workspace_diagnostics_include_fragment_path() {
    let output = compile_workspace_with_diagnostics(&workspace(
        "song.loom",
        "sections/intro.loom",
        &[
            (
                "song.loom",
                r#"---
fragments:
  intro: sections/intro.loom
---

# Lead: 1

[[intro]]
"#,
            ),
            ("sections/intro.loom", "# 1\nnot loom\n"),
        ],
    ));

    let PlaygroundCompileOutput::Err { diagnostics } = output else {
        panic!("expected diagnostics");
    };
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].path.as_deref(), Some("sections/intro.loom"));
    assert_eq!(diagnostics[0].line, Some(2));
    assert_eq!(diagnostics[0].column, Some(1));
}

#[test]
fn virtual_workspace_diagnostics_include_help_for_missing_fragment_mapping() {
    let output = compile_workspace_with_diagnostics(&workspace(
        "song.loom",
        "song.loom",
        &[(
            "song.loom",
            r#"---
fragments: {}
---

# Lead: 1

[[intro]]
"#,
        )],
    ));

    let PlaygroundCompileOutput::Err { diagnostics } = output else {
        panic!("expected diagnostics");
    };
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].path.as_deref(), Some("song.loom"));
    assert!(diagnostics[0].message.contains("Missing fragment mapping"));
    assert!(diagnostics[0]
        .help
        .as_deref()
        .is_some_and(|help| help.contains("fragments")));
}

#[test]
fn virtual_workspace_diagnostics_include_message_for_missing_fragment_file() {
    let output = compile_workspace_with_diagnostics(&workspace(
        "song.loom",
        "song.loom",
        &[(
            "song.loom",
            r#"---
fragments:
  intro: sections/intro.loom
---

# Lead: 1

[[intro]]
"#,
        )],
    ));

    let PlaygroundCompileOutput::Err { diagnostics } = output else {
        panic!("expected diagnostics");
    };
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].path.as_deref(), Some("song.loom"));
    assert!(diagnostics[0]
        .message
        .contains("Cannot read fragment 'sections/intro.loom'"));
}
