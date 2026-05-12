# Loom DSL Specification

## Top Level

```ebnf
Song        = [ Frontmatter ] { Track } ;
Frontmatter = "---" , newline , yaml_content , "---" , newline ;
Track       = TrackHeader , { Line } ;
TrackHeader = "#" , space , name , ":" , space , channel , [ space , "x" ] , newline ;
```

Frontmatter keys:

- `loop_range`: half-open unit range written as `start..end`.
  - `start` is inclusive and must be `>= 0`.
  - `end` is exclusive and must be greater than `start`.
  - Values are interpreted in the current `unit` (`bar` or `beat`).
  - Example: `loop_range: 0..1` loops the first bar when `unit: bar`.

## Lines

```ebnf
Line         = CommentLine | InitLine | PatternLine | SeqLine | EmptyLine | TrackWrap ;
CommentLine  = ">" , { character } , newline ;
InitLine     = "##" , space , InitCommand , newline ;
EmptyLine    = { space } , newline ;
TrackWrap    = "---" , newline ;
PatternLine  = RowHeader , Bar , Block , { Bar , Block } , Bar , [ space , CommentLine ] ;
SeqLine      = "seq" , space , Bar , SeqBlock , { Bar , SeqBlock } , Bar , [ space , CommentLine ] ;
```

## Patterns and Tokens

```ebnf
RowHeader   = NoteList | DrumName | MidiNote ;
NoteList    = NoteName , { "," , NoteName } ;
NoteName    = ( "a"..."g" | "A"..."G" ) , [ "#" | "b" ] , digit ;
MidiNote    = digits ;
DrumName    = "bd" | "kick" | "sn" | "snare" | "rs" | "rim" | "cp" | "clap"
            | "hh" | "hc" | "hihat" | "oh" | "ho" | "hp"
            | "cr" | "crash" | "rd" | "ride" | "splash" | "china"
            | "ht" | "mt" | "lt" | "ft" | "cb" | "tamb" ;

Bar         = "|" | "|:" | ":|" | ":|:" ;
Block       = { Token | space } ;
Token       = NoteOn | Rest | Sustain | Group ;
NoteOn      = "^" ;
Rest        = "." ;
Sustain     = "-" ;
Group       = "[" , { Token | space } , "]" ;
SeqBlock    = { SeqToken | space } ;
SeqToken    = SeqNote | Rest | Sustain | SeqGroup ;
SeqGroup    = "[" , { SeqToken | space } , "]" ;
SeqNote     = NoteName | Chord ;
Chord       = NoteName , "," , NoteName , { "," , NoteName } ;
```

Notes:

- `NoteName` is case-insensitive (for example, `c4` and `C4`).
- `MidiNote` must be `0..127`.
- `DrumName` is case-sensitive.
- `seq` is a sugar syntax: per-token notes/chords are written directly in the grid.
- `seq` currently requires explicit octave in each note (for example `C4`).
- For independent sustain per voice, use the standard pattern grid (`^ . -`) across separate rows.

Init command forms:

- `pc <0..127>` / `sound <0..127>`
- `bank <msb>/<lsb>`
- `cc <controller 0..127> <value 0..127>`
- `pan <0..127>` / `volume <0..127>` / `expression <0..127>` / `mod <0..127>` / `sustain <0..127>`

Whitespace handling note:

- Parsing is whitespace-tolerant around init/header tokens.
- Canonical spacing (for example `# Track: 1`, `## pc 4`) is defined by the formatter, not by the language acceptance rules.

## Modifier Lines

A modifier line adjusts per-token properties (for example velocity, pitch) for the immediately preceding pattern line.

```ebnf
ModifierLine   = ModifierKind , space , Bar , { ModifierEntry | space } , { Bar , { ModifierEntry | space } } , Bar ;
ModifierKind   = "v" | "p" ;
ModifierEntry  = ModifierValue | ModifierGroup | ModifierEmpty ;
ModifierValue  = [ "!" ] , [ "+" | "-" ] , digits ;
ModifierNoteList = [ "+" | "-" ] , digits , "," , [ "+" | "-" ] , digits , { "," , [ "+" | "-" ] , digits } ;
ModifierGroup  = "[" , { ModifierEntry | space } , "]" ;
ModifierEmpty  = "." ;
```

- `v` (Velocity): Absolute value (`0..127`). Default `100`.
- `p` (Pitch): Relative semitone offset (`+N` / `-N`). Default `0`.
- `!` (Latch): Value persists across later empty slots.
- `.` (Empty): Explicit empty slot (uses latch or default).
- Empty slot: Uses latch value if active, otherwise default.
- `[...]` (Group): Aligns sub-values 1:1 with pattern group sub-tokens.
- Scalar at group position: Broadcast to all leaves of that group.
- `100,80` note-list value is supported for `seq` chord tokens (length must match chord size).

## Templates

Templates define reusable sequences expanded in tracks.

### Template Definition

```ebnf
Template       = TemplateHeader , { Line } ;
TemplateHeader = "#" , space , "@" , name , newline ;
```

### Template Expansion

```ebnf
TemplateLine      = TemplateExpansion , { TemplateExpansion } , newline ;
TemplateExpansion = "[" , "@" , name , { space , TemplateParam } , "]" , [ "*" , digits ] ;
TemplateParam     = Transpose | StructuralRepeat | TimeScale | Macro ;
Transpose         = ( "+" | "-" ) , digits ;
StructuralRepeat  = "x" , digits ;
TimeScale         = "/" , digits ;
Macro             = "rev" | "arp" | "strum" | "vel:" , digits | "pan:" , digits ;
```

Rules:

- Multiple expansions on the same line are processed sequentially (`[@a][@b]`).
- Expansions on different lines in the same section are parallel by line semantics.
- `+N` / `-N`: pitch transposition.
- `xN`: structural repeat within the same grid span.
- `/N`: time scale (compress duration to `1/N`).
- `*N`: repeat the expanded sequence.
- Macros:
  - `rev`
  - `arp`
  - `strum`
  - `vel:N`
  - `pan:N`

## Lexical Rules

- `newline`: line ending (`\n`, `\r\n`)
- `space`: horizontal whitespace
- `yaml_content`: valid YAML string
- `name`: track/template name
- `channel`: MIDI channel (`1..16`)
- `character`: any UTF-8 character except newline
- `digit`: `0..9`

<!-- AUTO-GENERATED:DSL-SYMBOLS:START -->

## Symbol Table (Auto)

- `Note` => `^` - Note glyph
- `Rest` => `.` - Rest glyph
- `Sustain` => `-` - Sustain (tie) glyph
- `BarStandard` => `|` - Standard bar line
- `BarRepeatStart` => `|:` - Repeat start bar line
- `BarRepeatEnd` => `:|` - Repeat end bar line
- `BarDouble` => `:|:` - Double bar line / Section boundary
- `TrackHeader` => `#` - Track header start symbol
- `TrackHeaderSeparator` => `:` - Track header separator (name:channel)
- `TrackHeaderMute` => `x` - Track header mute flag
- `Comment` => `>` - Comment start symbol
- `TrackWrap` => `---` - Track wrap / Frontmatter boundary
- `GroupStart` => `[` - Group start
- `GroupEnd` => `]` - Group end
- `ModVelocity` => `v` - Velocity modifier selector
- `ModPitch` => `p` - Pitch modifier selector
- `ModLatch` => `!` - Modifier latch flag (!)
- `Positive` => `+` - Modifier relative positive value (+)
- `Negative` => `-` - Modifier relative negative value (-)
- `Template` => `@` - Template prefix symbol (@)

<!-- AUTO-GENERATED:DSL-SYMBOLS:END -->

## Track Wrapping

`---` in a track body works as a continuation marker.

```loom
# Track: 1
C3 | ^ - - | ^ . . |
---
C3 | ^ ^ ^ | ^ ^ ^ |
```

Equivalent timeline:

`C3 | ^ - - | ^ . . | ^ ^ ^ | ^ ^ ^ |`

## Global Configuration

See [Global Config](/reference/config).
