# CLI

## Commands

- `loom check <input>`
- `loom parse <input> [--format table|json|csv] [--sort ...] [--filter ...] [--summary]`
- `loom play <input> [--port N]`
- `loom live <input> [--port N]`
- `loom studio <input> [--port N]`
- `loom save <input> [output.mid]`
- `loom fmt [input] [--check]`
- `loom ports`

<!-- AUTO-GENERATED:CLI-COMMANDS:START -->

## Commands (Auto)

- `loom check`: Check syntax of Loom file (CI/CD, Validation)
- `loom parse`: Parse and output MIDI events (Dry run, formerly Run)
  - `input`
  - `--format`
  - `--sort`
  - `--filter`
  - `--summary`
- `loom play`: Real-time MIDI Playback (One-shot)
  - `input`
  - `-p, --port`
- `loom live`: Interactive Live Coding Mode (TUI & Hot-swap)
  - `input`
  - `-p, --port`
- `loom studio`: Integrated TUI composer
  - `input`
  - `-p, --port`
- `loom save`: Export to MIDI file
  - `input`
  - `output`
- `loom fmt`: Format Loom file
  - `input`
  - `-c, --check`
- `loom ports`: List available MIDI output ports

<!-- AUTO-GENERATED:CLI-COMMANDS:END -->

## Port Resolution

For `play`, `live`, and `studio`, the effective MIDI port is resolved in this order:

1. `--port`
2. global config (`~/.config/loom/loom.toml`)
3. default `0`

See [Global Config](/reference/config).

For local sound output, see [Playback](/guide/playback).

For the Studio workflow itself, see [Studio](/guide/studio).
