mod cli_ref;
mod diagnostics;
mod error_fixtures;
mod examples_index;
mod symbols;
mod template_macros;
mod util;

use std::fs;
use std::path::Path;

pub fn run(check_only: bool) -> Result<(), String> {
    let mut changed = Vec::new();

    let targets = vec![
        DocTarget {
            file: "site/reference/errors.md",
            start: "<!-- AUTO-GENERATED:ERROR-CODES:START -->",
            end: "<!-- AUTO-GENERATED:ERROR-CODES:END -->",
            content: diagnostics::build_diagnostic_codes()?,
        },
        DocTarget {
            file: "site/reference/errors.md",
            start: "<!-- AUTO-GENERATED:ERROR-FIXTURES:START -->",
            end: "<!-- AUTO-GENERATED:ERROR-FIXTURES:END -->",
            content: error_fixtures::build_error_fixture_samples()?,
        },
        DocTarget {
            file: "site/guide/cli.md",
            start: "<!-- AUTO-GENERATED:CLI-COMMANDS:START -->",
            end: "<!-- AUTO-GENERATED:CLI-COMMANDS:END -->",
            content: cli_ref::build_cli_reference()?,
        },
        DocTarget {
            file: "site/language/spec.md",
            start: "<!-- AUTO-GENERATED:DSL-SYMBOLS:START -->",
            end: "<!-- AUTO-GENERATED:DSL-SYMBOLS:END -->",
            content: symbols::build_dsl_symbols()?,
        },
        DocTarget {
            file: "site/language/templates.md",
            start: "<!-- AUTO-GENERATED:TEMPLATE-MACROS:START -->",
            end: "<!-- AUTO-GENERATED:TEMPLATE-MACROS:END -->",
            content: template_macros::build_template_macros()?,
        },
        DocTarget {
            file: "site/examples/index.md",
            start: "<!-- AUTO-GENERATED:EXAMPLES-INDEX:START -->",
            end: "<!-- AUTO-GENERATED:EXAMPLES-INDEX:END -->",
            content: examples_index::build_examples_overview()?,
        },
        DocTarget {
            file: "site/examples/starter.md",
            start: "<!-- AUTO-GENERATED:EXAMPLES-STARTER:START -->",
            end: "<!-- AUTO-GENERATED:EXAMPLES-STARTER:END -->",
            content: examples_index::build_examples_category("Starter", "starter", true)?,
        },
        DocTarget {
            file: "site/examples/musical.md",
            start: "<!-- AUTO-GENERATED:EXAMPLES-MUSICAL:START -->",
            end: "<!-- AUTO-GENERATED:EXAMPLES-MUSICAL:END -->",
            content: examples_index::build_examples_category("Musical", "musical", true)?,
        },
        DocTarget {
            file: "site/examples/live-coding.md",
            start: "<!-- AUTO-GENERATED:EXAMPLES-LIVE:START -->",
            end: "<!-- AUTO-GENERATED:EXAMPLES-LIVE:END -->",
            content: examples_index::build_examples_category("Live Coding", "live-coding", true)?,
        },
    ];

    for target in targets {
        let path = Path::new(target.file);
        let current = fs::read_to_string(path)
            .map_err(|e| format!("failed to read {}: {}", path.display(), e))?;
        let updated =
            util::replace_between_markers(&current, target.start, target.end, &target.content)?;
        if current != updated {
            if check_only {
                changed.push(target.file.to_string());
            } else {
                fs::write(path, updated)
                    .map_err(|e| format!("failed to write {}: {}", path.display(), e))?;
                println!("updated {}", target.file);
            }
        }
    }

    if check_only {
        if changed.is_empty() {
            println!("docs are up to date");
            return Ok(());
        }
        return Err(format!(
            "docs are out of date: {}. Run `cargo xtask gen-docs`.",
            changed.join(", ")
        ));
    }

    Ok(())
}

struct DocTarget {
    file: &'static str,
    start: &'static str,
    end: &'static str,
    content: String,
}
