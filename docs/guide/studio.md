# Studio

`loom studio` is the integrated TUI editor for `.loom` files.

It is optimized for a fast compose-listen-edit loop inside one terminal session:

- edit source directly in the TUI
- hear changes through the built-in player
- apply structure-aware editing operations
- save, format, and recompile without leaving the app

`loom live` remains the external-editor workflow. `loom studio` is the in-app workflow.

Studio is designed around modern terminals with Kitty keyboard protocol support. In practice, preview note input and other Shift/release-sensitive bindings are expected to work best in terminals such as Ghostty, Kitty, WezTerm, or Alacritty.

The long-term keybinding direction is documented in [Studio Keybinding Policy](/concepts/studio-keybindings).

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

Studio has four modes:

- `Normal`: navigation, editing commands, playback control
- `Insert`: free text editing
- `Select`: structure-aware range editing
- `Command`: short command-line edits and editor actions

### Normal Mode

Main bindings:

| Key | Behavior |
| --- | --- |
| `i` | enter Insert mode |
| `:` | enter Command mode |
| `a` | add prefix |
| `g` | goto prefix |
| `g d` | jump to definition: template definition under cursor, or mapped fragment file on a `[[fragment]]` line |
| `Ctrl-o` | return to the previous Studio file |
| `P` | open / close preview panel |
| `p` | paste yanked content after the current unit, bar, or template call |
| `n` | single note entry |
| `N` | continuous note entry |
| `o` | single onset entry |
| `O` | continuous onset entry |
| `m` | toggle mute on current track |
| `M` | toggle solo on current track |
| `X` | clear solo and mute flags on all tracks |
| `d` | structure delete prefix |
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

### Song Manifests

Studio can open a song manifest directly.

- `g d` on a `[[fragment]]` line opens the mapped fragment file.
- `Ctrl-o` returns to the previous Studio file and cursor position.
- Studio refuses file navigation while the current file is dirty. Save with `w` first.
- When editing a fragment opened from a manifest, compile/playback uses the manifest context and the current fragment buffer.
- Directly opening a fragment file has no manifest context; open the manifest first when you want full-song compile/playback.

### Command Mode

Command mode opens from Normal or Select mode with `:`. It uses the footer as a command prompt.

- `Enter` runs the command and returns to the previous mode
- `Esc` cancels and returns to the previous mode
- playback continues while entering commands
- invalid commands close Command mode and show an error message

Supported commands:

| Command | Behavior |
| --- | --- |
| `bpm 140` | set `bpm: 140` |
| `loop on` | set `loop: true` |
| `loop off` | set `loop: false` and keep `loop_range` |
| `loop clear` | remove `loop` and `loop_range` |
| `loop 0 4` | set `loop: true` and `loop_range: 0..4` |
| `w` | save |
| `q` | quit, with dirty warning |
| `q!` | force quit |
| `wq` | save and quit |
| `format` / `fmt` | format source |

`loop 0 4` uses the current frontmatter `unit`, matching `loop_range` semantics directly. Studio command edits only simple scalar frontmatter values. Complex YAML values should still be edited manually in Insert mode.

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
- `d` or `x` delete selected units or bars
- `y` yank selected units, bars, or template calls
- `p` paste yanked content after the current selection
- `g d` jumps from a selected template call to its definition, including library calls such as `[@alias.name]`
- transpose with `+`, `-`, `[`, `]` also applies to template call selection by editing the call's `+N` parameter
- `>` / `<` on template call selection adjust the call's `xN` repeat parameter
- `}` / `{` on template call selection adjust the call's `/N` time-scale parameter
- `n` replace selected units through note entry
- `s` subdivide selected units
- `S` shrink selected groups
- `T` extracts selected bars into a template definition
- `Enter` writes selected bars to `loop_range`

## Note Entry

Note entry is the fastest way to place notes in `seq` lines.

- `P` opens a preview panel for the current track without editing source
- `n` places one unit
- `N` keeps advancing to the next unit
- during `N`, `Space` skips to the next unit without writing, `Tab` subdivides the current unit, and `Shift-Tab` shrinks the current bracket group
- during `O`, `Space` skips to the next unit without writing, `Tab` subdivides the current unit, and `Shift-Tab` shrinks the current bracket group
- `Esc` exits note entry
- `Backspace` undoes the last step in continuous `N` / `O` entry
- inside the preview panel, note-on happens on key press and note-off happens on key release in Kitty-compatible terminals
- the preview panel is split into an LCD-style status row, performance pads for octave and preview `pc` changes, and a centered keyboard deck
- inside the preview panel, `[` / `]` adjust preview `pc` by 1, `{` / `}` adjust it by 10, and `r` resets to the source `pc`
- inside the preview panel, `Enter` applies the current preview `pc` to the track as `## pc`
- inside the preview panel, octave changes still work and `.` / `-` stay silent

Default keyboard layout:

```text
octave:
Z down / X up

black keys:
    W   E       T   Y   U       O   P
white keys:
  A   S   D   F   G   H   J   K   L   ;
pitch:
  C   D   E   F   G   A   B   C   D   E
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
| `a l` | add a lane |
| `a t` | add a new track and empty `seq` line |
| `a P` | add a new piano-roll-style track with `C4` through `B4` lanes |
| `a h` | add a `---` separator line |
| `a T` | add an empty template definition |
| `a b` | append a rest bar |
| `a d` | add the default drum preset |
| `a v` | add a velocity modifier line for the current pattern block |
| `a p` | add a pitch modifier line for the current pattern block |
| `a i` then `p` / `b` / `c` / `n` / `v` / `e` / `m` / `s` | add `## pc` / `bank` / `cc` / `pan` / `volume` / `expression` / `mod` / `sustain` |
| `a m` then `a` / `r` / `s` | add `arp` / `rev` / `strum` to the template call under cursor |
| `a n` | place a nearby or default note into the current `seq` slot |
| `a .` | place rest in the current `seq` slot |
| `a -` | place sustain in the current `seq` slot |

Modifier lines are selectable as units, so `v` / `p` lines support unit navigation plus Select-mode delete and yank / paste operations.

When the cursor is on a template call in Normal mode:

- `+`, `-`, `[`, and `]` edit that call's transpose parameter (`+N`)
- `>` and `<` edit that call's structural repeat parameter (`xN`)
- `}` and `{` edit that call's time-scale parameter (`/N`)
- library template calls such as `[@alias.name]` support the same call editing operations

Outside template calls, `<` and `>` keep their usual bar navigation behavior.

Onset editing for lane rhythm bodies:

- `o x` note-on
- `o .` rest
- `o -` sustain
- `o t` toggle between note-on and rest
- `O` repeats the same workflow continuously
- during `O`, `Space` skips ahead, `Tab` subdivides, `Shift-Tab` shrinks, and `Backspace` undoes the last step

## Track Operations

Studio can edit track header flags directly:

| Key | Behavior |
| --- | --- |
| `g t` | next track header |
| `g T` | previous track header |
| `m` | toggle current track mute flag `x` |
| `M` | toggle current track solo flag `s` |
| `X` | clear all track `s` / `x` flags |
| `D s` | delete current `seq` line |
| `D l` | delete current lane |
| `D t` | delete current track |
| `D h` | delete current `---` separator line |
| `D T` | delete current template definition block |
| `D b` | delete current bar on the line |
| `D v` | delete current velocity modifier line |
| `D p` | delete current pitch modifier line |
| `D m` | delete `arp` / `rev` / `strum` from the template call under cursor |
| `D i` then `p` / `b` / `c` / `n` / `v` / `e` / `m` / `s` | delete `## pc` / `bank` / `cc` / `pan` / `volume` / `expression` / `mod` / `sustain` |

Track header formatting is kept canonical:

- `# Name: channel`
- `# Name: channel s`
- `# Name: channel x`
- `# Name: channel s x`

## Song Settings

Currently supported frontmatter editing:

- `:` opens Command mode
- `bpm 140` sets `bpm`
- `loop on` / `loop off` edits `loop`
- `loop clear` removes `loop` and `loop_range`
- `loop 0 4` writes `loop_range: 0..4`
- bar selection + `T` extracts a template definition
- bar selection + `Enter` writes a `loop_range`

Studio only edits simple scalar frontmatter settings directly. Complex frontmatter should still be edited manually in Insert mode.

## Playback, Compile, And Audition

Behavior:

- Studio compiles on startup
- successful compile updates the playback source
- compile errors are shown in the Playback panel
- `space` toggles play / pause
- `r` restarts from the beginning

Studio also previews notes for many musical edits:

- `P` manual preview mode
- note entry
- onset placement
- transpose
- yank / paste in note-oriented contexts
- duplicate in note-oriented contexts

Manual preview mode opens a panel for the current track. It can audition notes and adjust initial playback controls before applying them to the source:

| Key | Action |
| --- | --- |
| `1` | select `pc` |
| `2` | select `volume` / CC7 |
| `3` | select `pan` / CC10 |
| `4` | select `expression` / CC11 |
| `5` | select `mod` / CC1 |
| `[` / `]` | adjust the selected value by `-1` / `+1` |
| `{` / `}` | adjust the selected value by `-10` / `+10` |
| `r` | reset unapplied preview changes to the source values |
| `Enter` | apply unapplied preview changes to the current track init |
| `Esc` / `P` | close the preview panel |

Applying preview changes writes canonical track init lines. `## sound` is normalized to `## pc`, and raw `## cc 7`, `## cc 10`, `## cc 11`, and `## cc 1` lines are normalized to `## volume`, `## pan`, `## expression`, and `## mod`.

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
