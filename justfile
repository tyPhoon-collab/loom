mod dev
mod docs

# List available recipes
default:
    @just --list --list-submodules

# Run the full CI check
ci: dev::ci

# Format a loom file
fmt-loom file *args:
    cargo run -- fmt {{ file }} {{ args }}

# Live-code a loom file
live file *args:
    cargo run -- live {{ file }} {{ args }}

# Open Studio for a loom file
studio file *args:
    cargo run -- studio {{ file }} {{ args }}

# Play a loom file
play file *args:
    cargo run -- play {{ file }} {{ args }}

# Parse a loom file
parse file *args:
    cargo run -- parse {{ file }} {{ args }}

# Save a loom file as MIDI
save file output="output.mid" *args:
    cargo run -- save {{ file }} {{ output }} {{ args }}

# List available MIDI output ports
ports:
    cargo run -- ports

# Pre-commit checks
precommit: dev::precommit
