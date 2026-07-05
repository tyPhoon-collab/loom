# Loom

Loom is a text-first DSL for MIDI composition, playback, and live coding.

## Quick Start

Loom sends MIDI to an external synth. For local playback, start FluidSynth first:

```bash
# Nix
nix run nixpkgs#fluidsynth -- -a coreaudio -m coremidi ~/Library/Audio/Sounds/Banks/FluidR3_GM.sf2

# Homebrew
brew install fluid-synth
fluidsynth -a coreaudio -m coremidi ~/Library/Audio/Sounds/Banks/FluidR3_GM.sf2
```

```bash
cargo build --release

# list MIDI ports
loom ports

# play once
loom play examples/starter/melody-simple.loom

# live coding (hot reload)
loom live examples/starter/melody-simple.loom

# export MIDI file
loom save examples/starter/melody-simple.loom
```

## Documentation

- Published docs: https://loom-5ue.pages.dev/
- Docs source: `site/` (VitePress)
- Playback setup: `site/guide/playback.md`
- AI-oriented overview: `site/llms.md`
- Some documentation sections are auto-generated from source-of-truth in code/tests.
  - Generate: `cargo xtask gen-docs` (or `just docs::gen`)
  - Verify up-to-date: `cargo xtask check-docs` (or `just docs::check`)

### Run Docs Locally

```bash
cd site
npm install
npm run docs:dev
```

## Repository Map

- `site/`: public user-facing documentation
  - `site/language/spec.md`: language specification
  - `site/reference/formatter.md`: formatter specification
  - `site/reference/config.md`: global config reference
  - `site/concepts/*`: concept and philosophy
- `docs/`: internal project documentation
  - `docs/adr/*`: architecture decision records
- `examples/`: runnable `.loom` examples
- `src/`: compiler, parser, runtime, commands
- `tests/`: integration and snapshot tests

## Top-level Files Policy

To keep the repository root focused and navigable:

- Keep only entrypoint documents at top level (for example `README.md`, `LICENSE`).
- Keep full user-facing documentation under `site/`.
- Keep internal project documentation under `docs/`.
- Avoid duplicating long-form docs between root, `site/`, and `docs/`.
- Keep implementation and tests in `src/` and `tests/`.
