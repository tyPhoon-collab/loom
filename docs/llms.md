# Loom for LLMs

This page provides a compact machine-oriented overview.

## Project Summary

- Loom is a text DSL for MIDI composition and live playback.
- The compiler outputs `MidiEvent` values:
  - `Note { time, duration, channel, note, velocity }`
  - `ControlChange { time, channel, cc, value }`
  - `ProgramChange { time, channel, program }`

## Important Paths

- language spec: `docs/language/spec.md`
- formatter spec: `docs/reference/formatter.md`
- examples: `examples/`
- parser/compiler runtime: `src/`

## Global Config

- fixed path: `~/.config/loom/loom.toml`
- keys:

```toml
[midi]
output_port = 0
```

- precedence for MIDI port:
  1. CLI `--port`
  2. config `midi.output_port`
  3. default `0`

## Current Doc Status

- VitePress docs are scaffolded and partially WIP.
- The `docs/` tree is the documentation source of truth.
