use loom::dsl::formatter;
use miette::{IntoDiagnostic, Result};
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

pub fn handle_fmt(input: Option<PathBuf>, check: bool) -> Result<()> {
    let (content, path_str) = match &input {
        Some(path) => (
            fs::read_to_string(path).into_diagnostic()?,
            path.display().to_string(),
        ),
        None => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer).into_diagnostic()?;
            (buffer, "<stdin>".to_string())
        }
    };

    let formatted = formatter::format_string(&content);

    if check {
        if content != formatted {
            eprintln!("Difference found in {}", path_str);
            std::process::exit(1);
        }
    } else {
        match input {
            Some(path) => {
                fs::write(&path, formatted).into_diagnostic()?;
                eprintln!("Formatted {}", path.display());
            }
            None => {
                print!("{}", formatted);
            }
        }
    }

    Ok(())
}
