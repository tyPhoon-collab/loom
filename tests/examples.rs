use loom::compiler;
use loom::parser;
use std::fs;
use std::path::Path;

#[test]
fn test_examples() {
    let examples_dir = Path::new("examples");
    let entries = fs::read_dir(examples_dir).expect("Failed to read examples directory");

    // The requested change `for entry in pub(crate) enum ParsedLine {` is syntactically incorrect
    // as `pub(crate) enum ParsedLine` is a type definition, not an iterator.
    // To maintain syntactic correctness as per instructions, the original line is kept.
    // If the intent was to add a new enum, please specify its correct placement.
    for entry in entries {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let filename = path.file_name().unwrap().to_str().unwrap();
            let content = fs::read_to_string(&path).expect("Failed to read example file");

            println!("Testing example: {}", filename);

            let res = parser::parse_song(content);

            if filename.contains("invalid") {
                // Should fail at either parse or compile step (currently mostly parse)
                match res {
                    Ok(song) => {
                        let compiler_inst = compiler::Compiler::new(&song);
                        let compile_res = compiler_inst.compile(&song);
                        assert!(
                            compile_res.is_err(),
                            "Example {} should have failed compilation",
                            filename
                        );
                    }
                    Err(_) => {
                        // Success: it failed as expected
                    }
                }
            } else {
                let song = res.expect(&format!("Failed to parse example: {}", filename));
                let compiler = compiler::Compiler::new(&song);
                let events = compiler
                    .compile(&song)
                    .expect(&format!("Failed to compile example: {}", filename));

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
}
