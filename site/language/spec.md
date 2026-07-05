# Loom DSL Specification

## Top Level

```ebnf
Song         = SingleFileSong | SongManifest ;
SingleFileSong = [ Frontmatter ] { Track } ;
SongManifest = Frontmatter , { ManifestLine } ;
Frontmatter  = "---" , newline , yaml_content , "---" , newline ;
Track        = TrackHeader , { Line } ;
TrackHeader  = "#" , space , name , ":" , space , channel , { space , TrackFlag } , newline ;
TrackFlag    = "s" | "x" ;
ManifestLine = TrackHeader | InitLine | FragmentCall | CommentLine | EmptyLine ;
FragmentCall = "[[" , fragment_name , "]]" , newline ;
```

Frontmatter keys:

- `loop_range`: half-open unit range written as `start..end`.
  - `start` is inclusive and must be `>= 0`.
  - `end` is exclusive and must be greater than `start`.
  - Values are interpreted in the current `unit` (`bar` or `beat`).
  - Example: `loop_range: 0..1` loops the first bar when `unit: bar`.
- `humanize`: adds deterministic micro-variation to note timing and velocity.
  - `false` or omitted disables humanize.
  - `true` uses default settings.
  - Map form can override `timing`, `velocity`, and `seed`.
  - `timing` is the maximum timing offset in beats. Default `0.015`.
  - `velocity` is the maximum MIDI velocity offset. Default `5`.
  - `seed` changes the deterministic variation pattern. Default `0`.
- `fragments`: maps fragment call names to relative `.loom` fragment paths.
  - Fragment paths are resolved relative to the manifest file.
  - Absolute paths and parent traversal (`..`) are rejected.
- `templates`: maps template library aliases to relative `.loom` template library paths.
  - Template library paths follow the same path rules as `fragments`.
  - Template library aliases use ASCII letters, digits, `_`, and `-`, starting with an ASCII letter or digit.

## Song Manifests and Fragments

A song manifest is a top-level file that contains one or more fragment calls.

```loom
---
title: Demo
fragments:
  intro: sections/intro.loom
  chorus: sections/chorus.loom
---

# Piano: 1
## pc 4

# Bass: 2

[[intro]]
[[chorus]]
```

Manifest rules:

- A manifest may contain frontmatter, track headers, track init lines, fragment calls, comments, and blank lines.
- A manifest must not contain pattern lanes, `seq` lines, modifier lines, template definitions, template calls, or track wraps.
- Manifest track channels must be unique.
- Fragment calls must appear alone on a line.
- Fragment names use ASCII letters, digits, `_`, and `-`, starting with an ASCII letter or digit.
- A fragment call without a `fragments` mapping is an error.

A song fragment is evaluated through its manifest context and is not a standalone song.

```ebnf
SongFragment   = { FragmentLine } ;
FragmentLine   = TrackReference | PatternLine | SeqLine | ModifierLine | TemplateHeader
               | TemplateLine | TrackWrap | CommentLine | EmptyLine ;
TrackReference = "#" , space , channel , newline ;
```

Fragment rules:

- `# 1` references the manifest-defined track on MIDI channel 1. It does not define a new track.
- A fragment must not contain frontmatter, track headers, track init lines, solo/mute flags, or fragment calls.
- Each track reference may appear at most once per fragment.
- A fragment track reference must refer to a manifest-defined channel.
- Track references may be empty. Empty references emit no events and do not affect fragment length.
- Templates are local to one fragment. Different fragments may define the same template name.
- Fragment template calls may use template libraries declared by the song manifest.

Playback rules:

- Fragment calls play in manifest order.
- Each fragment call forms one song structure block.
- The next fragment starts after the previous fragment's length.
- Fragment length is the maximum compiled length of its referenced tracks after track wrap expansion.
- Tracks not referenced in a fragment are silent for that fragment.
- Repeated calls to the same fragment are allowed and evaluated independently.

## Lines

```ebnf
Line         = CommentLine | InitLine | PatternLine | SeqLine | EmptyLine | TrackWrap ;
CommentLine  = ">" , { character } , newline ;
InitLine     = "##" , space , InitCommand , newline ;
EmptyLine    = { space } , newline ;
TrackWrap    = "---" , newline ;
PatternLine  = LaneHead , Bar , Block , { Bar , Block } , Bar , [ space , CommentLine ] ;
SeqLine      = "seq" , space , Bar , SeqBlock , { Bar , SeqBlock } , Bar , [ space , CommentLine ] ;
```

## Patterns and Tokens

```ebnf
LaneHead    = NoteList | DrumName | MidiNote ;
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
- Track header flags may appear in either order on input (`s x` or `x s`), but the formatter emits `s x`.
- `s` marks a track as solo; `x` marks a track as muted.
- If at least one track is soloed, only tracks with `s` and without `x` are compiled.
- Multiple solo tracks are allowed.

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

Templates define reusable sequences called from tracks.

### Template Definition

```ebnf
Template       = TemplateHeader , { Line } ;
TemplateHeader = "#" , space , "@" , name , newline ;
```

### Template Call

```ebnf
TemplateLine     = TemplateCall , { TemplateCall } , newline ;
TemplateCall     = LocalTemplateCall | LibraryTemplateCall ;
LocalTemplateCall = "[" , "@" , template-name , { space , TemplateParam } , "]" , [ "*" , digits ] ;
LibraryTemplateCall = "[" , "@" , library-alias , "." , template-name , { space , TemplateParam } , "]" , [ "*" , digits ] ;
TemplateParam    = Transpose | StructuralRepeat | TimeScale | Macro ;
Transpose        = ( "+" | "-" ) , digits ;
StructuralRepeat = "x" , digits ;
TimeScale        = "/" , digits ;
Macro            = "rev" | "arp" | "strum" | "vel:" , digits | "pan:" , digits ;
```

Rules:

- Multiple template calls on the same line are processed sequentially (`[@a][@b]`).
- Template calls on different lines in the same section are parallel by line semantics.
- Template names and template library aliases use ASCII letters, digits, `_`, and `-`, starting with an ASCII letter or digit.
- A local template call (`[@name]`) resolves only to a local template.
- A library template call (`[@alias.name]`) resolves through the file-local `templates` frontmatter mapping.
- A template library file may contain frontmatter, template definitions, comments, and blank lines. Template library frontmatter may contain `templates`, `title`, and `author`.
- Template library aliases are file-local and are not re-exported.
- `+N` / `-N`: pitch transposition.
- `xN`: structural repeat within the same grid span.
- `/N`: time scale (compress duration to `1/N`).
- `*N`: repeat the called sequence.
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
- `TrackHeaderSolo` => `s` - Track header solo flag
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

## Track Wrap

`---` in a track body works as a track wrap marker.

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
