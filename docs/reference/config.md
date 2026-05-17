# Global Configuration

## Path

Loom reads a single global config file:

- `~/.config/loom/loom.toml`

No alternate path is searched.

## Keys

```toml
[midi]
output_port = 0

[studio.note_keyboard]
base_octave = 4
octave_down = "z"
octave_up = "x"

[studio.note_keyboard.keys]
a = "C"
w = "C#"
s = "D"
e = "D#"
d = "E"
f = "F"
t = "F#"
g = "G"
y = "G#"
h = "A"
u = "A#"
j = "B"
k = "C+1"
o = "C#+1"
l = "D+1"
"." = "rest"
"-" = "sustain"
```

- `midi.output_port`: default MIDI output port index.
- `studio.note_keyboard.base_octave`: starting octave for Studio note entry.
- `studio.note_keyboard.octave_down`: normal-mode key for lowering the note keyboard octave.
- `studio.note_keyboard.octave_up`: normal-mode key for raising the note keyboard octave.
- `studio.note_keyboard.keys`: note entry keyboard map.

`studio.note_keyboard.keys` values are relative to `base_octave`:

- `"C"`: C in the base octave.
- `"C#"`: C sharp in the base octave.
- `"C+1"`: C one octave above the base octave.
- `"D-1"`: D one octave below the base octave.
- `"rest"`: place a rest token (`.`).
- `"sustain"`: place a sustain token (`-`).

Unsupported note names, multi-character keys, and invalid octave offsets are ignored.

## Precedence

1. CLI option (`--port`)
2. global config (`midi.output_port`)
3. built-in default (`0`)

## Scope

Applied to:

- `loom play`
- `loom live`
- `loom studio`

Not applied to song semantics (`bpm`, `signature`, `unit`, `swing`, etc.).
