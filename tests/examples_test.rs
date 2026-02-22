use loom::compiler;
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
            let events = compiler
                .compile(&song)
                .unwrap_or_else(|_| panic!("Failed to compile example: {}", filename));

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
