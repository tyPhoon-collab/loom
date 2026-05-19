# Studio

`loom studio` is the integrated TUI editor for `.loom` files.

It is optimized for a fast compose-listen-edit loop inside one terminal session:

- edit source directly in the TUI
- hear changes through the built-in player
- apply structure-aware editing operations
- save, format, and recompile without leaving the app

`loom live` remains the external-editor workflow. `loom studio` is the in-app workflow.

## Launch

```bash
loom studio path/to/song.loom
loom studio path/to/song.loom --port 1
```

The effective MIDI port is resolved in this order:

1. `--port`
2. global config
3. built-in default `0`

See [Global Config](/reference/config) and [Playback](/guide/playback).

## Layout

Studio has three areas:

- `Score`: the current `.loom` source
- `Playback`: device, transport, beat, BPM, compile status, target, and messages
- `Footer`: mode-specific key help

The score title also shows:

- current mode
- dirty state
- cursor target

## Modes

Studio has three modes:

- `Normal`: navigation, editing commands, playback control
- `Insert`: free text editing
- `Select`: structure-aware range editing

### Normal Mode

Main bindings:

| Key | Behavior |
| --- | --- |
| `i` | enter Insert mode |
| `a` | add prefix |
| `g` | goto prefix |
| `g d` | jump to template definition under cursor |
| `n` | single note entry |
| `N` | continuous note entry |
| `o` | single onset entry |
| `O` | continuous onset entry |
| `m` | toggle mute on current track |
| `M` | toggle solo on current track |
| `X` | clear solo and mute flags on all tracks |
| `D` | structure delete prefix |
| `x` | delete current unit |
| `s` | subdivide current unit |
| `S` | shrink current bracket group |
| `,` / `.` | previous / next unit |
| `<` / `>` | previous / next bar |
| `v` | select current unit |
| `V` | select current line |
| `b` | select current bar |
| `B` | select all bars on current line |
| `+` / `=` | transpose +1 semitone |
| `-` | transpose -1 semitone |
| `]` | transpose +12 semitones |
| `[` | transpose -12 semitones |
| `L` | toggle `loop` |
| `Ctrl-L` | clear `loop` and `loop_range` |
| `space` | play / pause |
| `r` | restart playback |
| `f` | format |
| `w` | save |
| `u` | undo |
| `R` | redo |
| `q` | quit, with dirty warning |
| `Q` | force quit |

### Insert Mode

Insert mode delegates text editing to the textarea.

- `Esc` returns to Normal mode
- leaving Insert mode recompiles the song
- Studio does not auto-format while you type

### Select Mode

Select mode works on Loom-aware units instead of raw text selection.

Supported selections:

- unit selection
- unit range selection
- template call selection
- template call range selection
- line range selection
- bar range selection

Typical Select mode operations:

- move / expand selection with `hjkl` and `HJKL`
- `x` delete selected units or bars
- `d` duplicate selected units or bars
- `g d` jumps from a selected template call to its definition
- transpose with `+`, `-`, `[`, `]` also applies to template call selection by editing the call's `+N` parameter
- `n` replace selected units through note entry
- `s` subdivide selected units
- `S` shrink selected groups
- `T` extracts selected bars into a template definition
- `Enter` writes selected bars to `loop_range`

## Note Entry

Note entry is the fastest way to place notes in `seq` lines.

- `n` places one unit
- `N` keeps advancing to the next unit
- during `N`, `Tab` subdivides the current unit and `Shift-Tab` shrinks the current bracket group
- `Esc` exits note entry
- `Backspace` undoes the last step in continuous note entry

Default keyboard layout:

```text
octave:
Z down / X up

black keys:
    W   E       T   Y   U       O
white keys:
  A   S   D   F   G   H   J   K   L
pitch:
  C   D   E   F   G   A   B   C   D
```

Notes:

- octave switching with `z` / `x` is available during note entry, not in Normal mode
- Select mode note replacement also accepts `z` / `x` for octave changes
- `.` places rest
- `-` places sustain

The note keyboard is configurable through `[studio.note_keyboard]`. See [Global Config](/reference/config).

## Add And Grid Editing

Useful add commands:

| Key | Behavior |
| --- | --- |
| `a s` | add a `seq` line |
| `a l` | add a note-head line |
| `a t` | add a new track and empty `seq` line |
| `a h` | add a `---` separator line |
| `a T` | add an empty template definition |
| `a b` | append a rest bar |
| `a d` | add the default drum preset |
| `a v` | add a velocity modifier line for the current pattern block |
| `a p` | add a pitch modifier line for the current pattern block |
| `a m` then `a` / `r` / `s` | add `arp` / `rev` / `strum` to the template call under cursor |
| `a n` | place a nearby or default note into the current `seq` slot |
| `a .` | place rest in the current `seq` slot |
| `a -` | place sustain in the current `seq` slot |

Modifier lines are selectable as units, so `v` / `p` lines support unit navigation plus Select-mode delete and duplicate operations.

When the cursor is on a template call in Normal mode, `+`, `-`, `[`, and `]` edit that call's transpose parameter (`+N`) instead of transposing line text.

Onset editing for note-head and drum-lane rhythm bodies:

- `o x` note-on
- `o .` rest
- `o -` sustain
- `o t` toggle between note-on and rest
- `O` repeats the same workflow continuously

## Track Operations

Studio can edit track header flags directly:

| Key | Behavior |
| --- | --- |
| `g t` | next track header |
| `g T` | previous track header |
| `m` | toggle current track mute flag `x` |
| `M` | toggle current track solo flag `s` |
| `X` | clear all track `s` / `x` flags |
| `D t` | delete current track |

Track header formatting is kept canonical:

- `# Name: channel`
- `# Name: channel s`
- `# Name: channel x`
- `# Name: channel s x`

## Song Settings

Currently supported frontmatter editing:

- `L` toggles `loop`
- `Ctrl-L` clears `loop` and `loop_range`
- bar selection + `T` extracts a template definition
- bar selection + `Enter` writes a `loop_range`

Studio only edits simple scalar loop settings directly. Complex frontmatter should still be edited manually in Insert mode.

## Playback, Compile, And Audition

Behavior:

- Studio compiles on startup
- successful compile updates the playback source
- compile errors are shown in the Playback panel
- `space` toggles play / pause
- `r` restarts from the beginning

Studio also previews notes for many musical edits:

- note entry
- onset placement
- transpose
- duplicate in note-oriented contexts

Preview is suppressed while normal playback is already running.

## Save, Format, Undo

- `w` saves and recompiles
- `f` formats the current source
- formatting marks the buffer dirty
- `u` undoes edits
- `R` redoes edits

Studio does not auto-save and does not auto-format on every edit.

## Current Scope

`loom studio` is already usable as an MVP, but it is still evolving.

Current strengths:

- tight edit/playback loop
- fast `seq` note entry
- track-level mute/solo editing
- structure-aware selection and transforms

Still best treated as an experimental interface:

- some workflows are more complete than others
- Insert mode is still important for raw text edits
- more settings and track commands will likely be added over time
