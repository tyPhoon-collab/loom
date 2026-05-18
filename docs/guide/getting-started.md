# Getting Started

## Install

Rust toolchain is required.

```bash
cargo build --release
```

## Basic Commands

Start a MIDI synth first. See [Playback](/guide/playback) for the recommended
FluidSynth setup.

```bash
# list MIDI ports
loom ports

# one-shot playback
loom play examples/starter/melody-simple.loom

# live coding (hot reload)
loom live examples/starter/melody-simple.loom

# integrated TUI composer
loom studio examples/starter/melody-simple.loom

# save as .mid
loom save examples/starter/melody-simple.loom
```

## Next

- Learn the [CLI](/guide/cli)
- Learn [Studio](/guide/studio)
- Set up [Playback](/guide/playback)
- Learn the [Language Specification](/language/spec)
