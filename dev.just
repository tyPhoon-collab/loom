# Build the Rust project
build:
    cargo build

# Check the Rust project
check:
    cargo check

# Run clippy with warnings denied
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Run Rust tests
test:
    cargo test

# Format Rust code
fmt-rust:
    cargo fmt

# Run the full CI check
ci: fmt-rust check clippy test

# Pre-commit checks
precommit:
    lefthook run pre-commit

# Setup for dev
setup:
    lefthook install
