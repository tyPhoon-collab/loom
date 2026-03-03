pub mod docs;

pub fn run(cmd: &str) -> Result<(), String> {
    match cmd {
        "gen-docs" => docs::run(false),
        "check-docs" => docs::run(true),
        _ => {
            eprintln!("Usage:");
            eprintln!("  cargo xtask gen-docs");
            eprintln!("  cargo xtask check-docs");
            Ok(())
        }
    }
}
