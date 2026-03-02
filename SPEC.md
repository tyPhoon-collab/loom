# Loom DSL Specification

## Top Level
```ebnf
Song        = [ Frontmatter ] { Track } ;
Frontmatter = "---" , newline , yaml_content , "---" , newline ;
Track       = TrackHeader , { Line } ;
TrackHeader = "#" , space , name , ":" , space , channel , [ space , "x" ] , newline ;
```

## Lines
```ebnf
Line         = CommentLine | PatternLine | EmptyLine | TrackWrap ;
CommentLine  = ">" , { character } , newline ;
EmptyLine    = { space } , newline ;
TrackWrap    = "---" , newline ;
PatternLine = RowHeader , Bar , Block , { Bar , Block } , Bar , [ space , CommentLine ] ;
```

## Patterns & Tokens
```ebnf
RowHeader   = NoteList | DrumName | MidiNote ;
NoteList    = NoteName , { "," , NoteName } ;
NoteName    = ( "a"..."g" | "A"..."G" ) , [ "#" | "b" ] , digit ;
MidiNote    = digits ;
DrumName    = "bd" | "kick" | "sn" | "snare" | "rs" | "rim" | "cp" | "clap"
            | "hh" | "hc" | "hihat" | "oh" | "ho" | "hp"
            | "cr" | "crash" | "rd" | "ride" | "splash" | "china"
            | "ht" | "mt" | "lt" | "ft" | "cb" | "tamb" ;

> [!NOTE]
> - **NoteName** is case-insensitive (e.g., `c4` and `C4` are the same).
> - **MidiNote** must be a valid number from `0` to `127` (e.g., `60` represents Middle C).
> - **DrumName** is case-sensitive (e.g., `kick` is valid, `KICK` is not).

Bar         = "|" | "|:" | ":|" | ":|:" ;
Block       = { Token | space } ;
Token       = NoteOn | Rest | Sustain | Group ;
NoteOn      = "^" ;
Rest        = "." ;
Sustain     = "-" ;
Group       = "[" , { Token | space } , "]" ;
```

## Modifier Lines

A modifier line adjusts per-token properties (e.g. velocity, pitch) for the immediately preceding pattern line.

```ebnf
ModifierLine   = ModifierKind , space , Bar , { ModifierEntry | space } , { Bar , { ModifierEntry | space } } , Bar ;
ModifierKind   = "v" | "p" ;
ModifierEntry  = ModifierValue | ModifierGroup | ModifierEmpty ;
ModifierValue  = [ "!" ] , [ "+" | "-" ] , digits ;
ModifierGroup  = "[" , { ModifierEntry | space } , "]" ;
ModifierEmpty  = "." ;
```

- **`v`** (Velocity): Absolute value (0–127). Default: 100.
- **`p`** (Pitch): Relative semitone offset (`+N` / `-N`). Default: 0.
- **`!` prefix (Latch)**: The value persists for subsequent empty slots.
- **`.` (Empty)**: Explicitly marks a slot as empty (uses latch or default). Equivalent to a whitespace-only gap, but useful inside `[...]` groups.
- **Empty slot**: Uses the latched value if active, otherwise the default.
- **`[...]` (Group)**: Aligns sub-values 1:1 with the sub-tokens of the corresponding pattern `[...]`. Empty slots within a group use the latch/default rules.
- **Scalar at Group position**: When a single modifier value corresponds to a pattern `[...]`, the value is broadcast to all sub-tokens.

```loom
C3 | ^    ^    ^    ^   |
v  | !80            100 |
p  | +2                 |
```

| Slot | Velocity | Pitch | Reason |
|------|----------|-------|--------|
| 1    | 80       | +2    | `!80` latch, `+2` one-shot |
| 2    | 80       | 0     | latch continues, default |
| 3    | 80       | 0     | latch continues, default |
| 4    | 100      | 0     | `100` one-shot (latch released) |

## Templates

Templates allow you to define reusable patterns that can be expanded within tracks.

### Template Definition
A template is defined similarly to a track, but the header starts with `@`.

```ebnf
Template       = TemplateHeader , { Line } ;
TemplateHeader = "#" , space , "@" , name , newline ;
```

Templates are not directly rendered; they must be expanded within a track.

### Template Expansion
Templates are expanded using the `[@name]` syntax. They can take parameters for transposition, structural repetition, and macros.

```ebnf
TemplateLine      = TemplateExpansion , { TemplateExpansion } , newline ;
TemplateExpansion = "[" , "@" , name , { "|" , TemplateParam } , "]" , [ "*" , digits ] ;
TemplateParam     = Transpose | StructuralRepeat | TimeScale | Macro ;
Transpose         = ( "+" | "-" ) , digits ;
StructuralRepeat  = "x" , digits ;
TimeScale         = "/" , digits ;
Macro             = "rev" | "arp" | "strum" | "vel:" , digits ;
```

- **Sequential Processing**: When multiple template expansions are written on the same line (e.g., `[@a][@b]`), they are processed **sequentially** (B starts after A finishes). This is different from expansions on separate lines, which are processed **parallelly** at the start of the section.

- **Transpose (`+N` / `-N`)**: Shifts the pitch of all notes in the template by `N` semitones.
- **Structural Repeat (`xN`)**: Repeats the content within the same grid duration. For example, a single `^` with `x4` becomes `^ ^ ^ ^` within the same total time.
- **Time Scale (`/N`)**: Compresses the template playback to `1/N` of its original duration. Propagates through nested template calls.
- **Sequence Repeat (`*N`)**: Repeats the entire template `N` times.
- **Macros**:
    - `rev`: Reverses the sequence.
    - `arp`: Arpeggiates simultaneous notes, spreading them evenly across the block duration.
    - `strum`: Adds slight timing offsets between simultaneous notes (guitar strum feel).
    - `vel:N`: Overrides velocity for all notes in the template (0–127).

```loom
# Track: 1
[@4beat]*2
[@maj|x4][@maj|+5|x4]

# @4beat
hh | ^ ^ ^ ^ |
sn | .   ^   |
bd | ^   ^   |

# @maj
C3,E3,G3 | ^ |
```

## Lexical Rules
- `newline`: Line ending (`\n`, `\r\n`).
- `space`: Horizontal whitespace.
- `yaml_content`: Valid YAML string.
- `name`: Track or template name (string).
- `channel`: MIDI channel (1-16). Note: Following the General MIDI standard, channel 10 is reserved for drums/percussion.
- `character`: Any UTF-8 character except newline.
- `digit`: `0`..."9".
- `alphabetic`: `a`..."z" | `A`..."Z".

## Track Wrapping
Loom allows breaking long timelines across multiple text blocks to improve readability.
This is achieved using the `TrackWrap` (`---`) within the body.

```loom
# Track: 1
C3 | ^ - - | ^ . . |
---
C3 | ^ ^ ^ | ^ ^ ^ |
```
This expands to: `C3 | ^ - - | ^ . . | ^ ^ ^ | ^ ^ ^ |`

The active track context is maintained across `---` boundaries. For readability and simplicity, it is recommended to write out an entire track (including all its wrapped sections) before moving on to define the next track, rather than interleaving sections of different tracks.

## Formatter

See [FORMATTER.md](FORMATTER.md) for the full formatter specification.

## Global Configuration

Loom supports a single global config file:

- Path: `~/.config/loom/loom.toml`
- This path is fixed. Loom does not search alternate locations.

Current keys:

```toml
[midi]
output_port = 0
```

- `midi.output_port`: default MIDI output port index (`usize`).

Precedence:

1. CLI option (`--port`)
2. Global config (`midi.output_port`)
3. Built-in default (`0`)

Scope:

- Applied to: `play`, `live`
- Not applied to: song/frontmatter semantics (`bpm`, `signature`, `unit`, `swing`, etc.)

If the file is missing or invalid, Loom falls back to defaults.
