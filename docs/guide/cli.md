# CLI

## Commands

- `loom check <input>`
- `loom parse <input> [--format table|json|csv] [--sort ...] [--filter ...] [--summary]`
- `loom play <input> [--port N]`
- `loom live <input> [--port N]`
- `loom save <input> [output.mid]`
- `loom fmt [input] [--check]`
- `loom ports`

## Port Resolution

For `play` and `live`, the effective MIDI port is resolved in this order:

1. `--port`
2. global config (`~/.config/loom/loom.toml`)
3. default `0`

See [Global Config](/reference/config).
