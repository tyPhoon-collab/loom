# Global Configuration

## Path

Loom reads a single global config file:

- `~/.config/loom/loom.toml`

No alternate path is searched.

## Keys

```toml
[midi]
output_port = 0
```

- `midi.output_port`: default MIDI output port index.

## Precedence

1. CLI option (`--port`)
2. global config (`midi.output_port`)
3. built-in default (`0`)

## Scope

Applied to:

- `loom play`
- `loom live`

Not applied to song semantics (`bpm`, `signature`, `unit`, `swing`, etc.).
