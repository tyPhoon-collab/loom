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
fmt-loom file *args:
    cargo run -- fmt {{file}} {{args}}

live file *args:
    cargo run -- live {{file}} {{args}}

play file *args:
    cargo run -- play {{file}} {{args}}

parse file *args:
    cargo run -- parse {{file}} {{args}}

save file output="output.mid" *args:
    cargo run -- save {{file}} {{output}} {{args}}

ports:
    cargo run -- ports

# Pre-commit checks
precommit:
    lefthook run pre-commit

# Setup for dev
setup:
    lefthook install