# Loom

Loom is a text-first DSL for MIDI composition, playback, and live coding.

## Quick Start

```bash
cargo build --release

# list MIDI ports
loom ports

# play once
loom play examples/melody-simple.loom

# live coding (hot reload)
loom live examples/melody-simple.loom

# export MIDI file
loom save examples/melody-simple.loom
```

## Documentation

- Docs source: `docs/` (VitePress)
- AI-oriented overview: `llms.txt` and `docs/llms.md`
- Some documentation sections are auto-generated from source-of-truth in code/tests.
  - Generate: `cargo xtask gen-docs` (or `just docs-gen`)
  - Verify up-to-date: `cargo xtask check-docs` (or `just docs-check`)

### Run Docs Locally

```bash
cd docs
npm install
npm run docs:dev
```

## Repository Map

- `docs/`: all user-facing documentation
  - `docs/language/spec.md`: language specification
  - `docs/reference/formatter.md`: formatter specification
  - `docs/reference/config.md`: global config reference
  - `docs/concepts/*`: concept and philosophy
- `examples/`: runnable `.loom` examples
- `src/`: compiler, parser, runtime, commands
- `tests/`: integration and snapshot tests
- `llms.txt`: machine entrypoint for doc discovery

## Top-level Files Policy

To keep the repository root focused and navigable:

- Keep only entrypoint documents at top level (for example `README.md`, `LICENSE`, `llms.txt`).
- Keep full user-facing documentation under `docs/`.
- Avoid duplicating long-form docs between root and `docs/`.
- Keep implementation and tests in `src/` and `tests/`.
