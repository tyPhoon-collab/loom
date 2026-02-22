use loom::compiler;
use loom::dsl::parser;
use miette::Report;
use std::fs;
use std::path::Path;

#[test]
fn test_errors() {
    let errors_dir = Path::new("tests/fixtures/errors/input");
    if !errors_dir.exists() {
        return;
    }

    let entries = fs::read_dir(errors_dir).expect("Failed to read error fixtures directory");
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "loom"))
        .collect();

    // Sort to ensure deterministic snapshot order
    files.sort_by_key(|e| e.path());

    for entry in files {
        let path = entry.path();
        let filename = path.file_name().unwrap().to_str().unwrap();
        let content = fs::read_to_string(&path).expect("Failed to read error fixture file");

        println!("Testing error snapshot: {}", filename);

        let result = parser::parse_song(content)
            .map_err(Report::new)
            .and_then(|song| {
                let compiler_inst =
                    compiler::Compiler::new(&song).expect("Failed to create compiler");
                compiler_inst.compile(&song)
            });

        match result {
            Ok(_) => {
                panic!(
                    "Expected {} to fail, but it succeded! If this is intended, move it to examples/.",
                    filename
                );
            }
            Err(e) => {
                // Render the miette Report into a plain string without ANSI color codes
                let mut error_trace = String::new();
                miette::GraphicalReportHandler::new()
                    .with_theme(miette::GraphicalTheme::unicode_nocolor())
                    .render_report(&mut error_trace, e.as_ref())
                    .unwrap();

                // Write snapshot to tests/snapshots/...
                insta::with_settings!({
                    snapshot_path => "snapshots",
                    prepend_module_to_snapshot => false,
                }, {
                    insta::assert_snapshot!(filename, error_trace);
                });
            }
        }
    }
}
