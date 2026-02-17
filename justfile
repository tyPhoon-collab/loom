# Build and run
default:
    @just --list

build:
    cargo build

check:
    cargo check

clippy:
    cargo clippy -- -D warnings

test:
    cargo test

fmt-rust:
    cargo fmt

# Loom Commands
fmt-loom file:
    cargo run -- fmt {{file}}

live file:
    cargo run -- live {{file}}

parse file:
    cargo run -- parse {{file}}

save file output="output.mid":
    cargo run -- save {{file}} {{output}}

# Pre-commit checks
precommit:
    lefthook run pre-commit

# Setup for dev
setup:
    lefthook install