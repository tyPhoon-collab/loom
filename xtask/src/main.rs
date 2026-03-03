fn main() -> Result<(), String> {
    let cmd = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "help".to_string());
    xtask::run(&cmd)
}
