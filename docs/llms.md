# Loom for LLMs

This document is intentionally self-contained.
It is written for AI agents that need to read, edit, or generate Loom files without relying on any other documentation.

## What Loom Is

- Loom is a text-first DSL for MIDI composition and live coding.
- A song is either a single `.loom` file or a manifest `.loom` file that calls fragment `.loom` files.
- Tracks contain lanes, `seq` sugar lines, modifier lines, init lines, comments, template definitions, template calls, and `---` track wrap markers.
- The compiler emits MIDI `Note`, `ControlChange`, and `ProgramChange` events.
- Track channels are written as 1-based numbers in source and compiled to 0-based MIDI channels.

## Minimal Shape

```loom
---
bpm: 120
signature: 4/4
unit: bar
---

# Piano: 1
## pc 4
## pan 64

C4 | ^ . ^ . |
```

## Frontmatter

Frontmatter is YAML between leading and trailing `---` lines.

Common keys currently supported:

- `bpm`: tempo, valid range `1..999`, default `120`
- `signature`: time signature string like `4/4`, default `4/4`
- `unit`: `bar` or `beat`, default `bar`
- `pitch`: global semitone transposition, default `0`
- `loop`: boolean loop flag, default `false`
- `loop_range`: half-open range like `0..1`, interpreted in the current `unit`
- `humanize`: `false`, `true`, or a map with `timing`, `velocity`, `seed`
- `swing`: `false`/omitted, a numeric grid like `8`, or a map like `{ grid: 8, amount: 66 }`
- `fragments`: manifest-only map from fragment call name to relative `.loom` path
- `title` and `author`: metadata only

Frontmatter rules:

- `loop_range` must satisfy `start >= 0` and `end > start`.
- `humanize: true` uses the default humanize settings.
- Default humanize settings are deterministic: `timing: 0.015`, `velocity: 5`, `seed: 0`.
- `swing: true` behaves like an 8th-note swing with amount `66`.
- `swing: 8` and `swing: { grid: 8, amount: 66 }` are both valid.
- `pitch` shifts pitched notes globally before compilation.
- Fragment paths must be relative and must not contain `..`.

Example:

```loom
---
bpm: 120
signature: 4/4
unit: bar
pitch: 12
loop: true
loop_range: 0..2
humanize:
  timing: 0.015
  velocity: 5
  seed: 42
swing: 8
---
```

## Manifests and Fragments

Use a manifest for large songs split into sections.

Manifest:

```loom
---
fragments:
  intro: sections/intro.loom
  chorus: sections/chorus.loom
---

# Piano: 1
## pc 4

# Drums: 10

[[intro]]
[[chorus]]
```

Fragment:

```loom
# 1
C4 | ^ . ^ . |

# 10
kick  | ^ . . . |
snare | . . ^ . |
```

Manifest rules:

- Fragment calls are `[[name]]` alone on a line.
- Manifest files may contain only frontmatter, track headers, track init lines, fragment calls, comments, and blank lines.
- Track channels must be unique in a manifest.
- No patterns, `seq`, modifiers, templates, template calls, or track wraps in a manifest.

Fragment rules:

- `# 1` is a track reference to manifest channel 1, not a track definition.
- Fragments must not contain frontmatter, track headers like `# Piano: 1`, track init lines, solo/mute flags, or fragment calls.
- Each channel may be referenced at most once per fragment.
- Templates are local to one fragment; same template name may be reused in another fragment.
- Fragment calls play in manifest order. Missing tracks are silent for that fragment.

## Track Headers

Track headers start with `#` and declare a name and MIDI channel:

```loom
# Piano: 1
# Drums: 10 s
# Lead: 2 s x
```

Rules:

- The channel is 1-based in source.
- `s` means solo.
- `x` means muted.
- Flags may appear in either order in input, but the formatter canonicalizes to `s x`.
- If any track is soloed, only solo tracks that are not muted are compiled.
- Muted tracks never compile, even if they are also soloed.

The track name is everything before the `:` separator, trimmed by the parser.

## Line Types

Inside a track body, the parser accepts:

- Comment lines starting with `>`
- Track init lines starting with `##`
- Lanes starting with a lane head like `C4`, `kick`, or `60`
- `seq` lines, which are sugar for note literals directly in the grid
- Modifier lines starting with `v` or `p`
- Template definition headers starting with `# @name`
- Template calls like `[@name ...]`
- `---` track wrap markers
- Blank lines

Trailing comments after pattern and modifier lines are supported:

```loom
C4 | ^ . ^ . |  > trailing comment
v  | 80 . 90 |  > trailing comment
```

Comments start with `>` only. `#` is not a comment marker inside normal track content.

## Lanes

Lanes are pattern rows written as a lane head plus bar-delimited blocks.

```loom
C4        | ^ . ^ . |
kick      | ^ . ^ . |
C3,E3,G3  | ^ . ^ . |
60        | ^ . ^ . |
```

Lane head forms:

- Pitch names like `C4`, `db3`, `F#2`
- Drum aliases like `kick`, `snare`, `hh`, `ride`
- Comma-separated pitch lists like `C3,E3,G3`
- MIDI note numbers like `60`

Pitch parsing rules:

- Pitch names are case-insensitive.
- Drum aliases are case-sensitive.
- Numeric note literals must be in `0..127`.
- If a pitch omits the octave, octave `3` is assumed by the current parser.

Supported drum aliases:

- `bd`, `kick`
- `sn`, `snare`
- `rs`, `rim`
- `cp`, `clap`
- `hh`, `hc`, `hihat`
- `oh`, `ho`
- `hp`
- `cr`, `crash`
- `rd`, `ride`
- `splash`
- `china`
- `ht`, `mt`, `lt`, `ft`
- `cb`
- `tamb`

## Bars

The bar markers are:

- `|` standard bar
- `|:` repeat start
- `:|` repeat end
- `:|:` double bar / section boundary

Blocks are delimited by bars.
The parser expects each non-empty block to end with a bar.

`---` inside a track body is not a bar. It is a track wrap marker that continues the current track across sections.

Example:

```loom
# Piano: 1
C3 | ^ - - |
---
C3 | ^ ^ ^ |
```

## Core Tokens

Inside a pattern block, the core tokens are:

- `^` note onset
- `.` rest
- `-` sustain / tie
- `[...]` group

Groups can nest.
The formatter treats a group as a single outer token for alignment.

Practical rules:

- Use separate rows when you want independent sustain per voice.
- Use groups when you want one slot split into multiple sub-tokens.

Example:

```loom
C4 | ^ - - |
C3 | ^ [^ ^] . |
```

## `seq` Sugar

`seq` is a shorthand syntax where the grid contains note literals directly.

```loom
seq | C4 D4 E4 . | [G4,B4 D5,E5] . C5 - |
```

Supported `seq` token forms:

- note literals like `C4`
- chord literals like `C4,E4,G4`
- `.` rest
- `-` sustain
- `[...]` group

Important details:

- `seq` is lowercase `seq` only.
- Chord literals in `seq` are comma-separated and must not contain spaces inside the literal.
- A chord literal is parsed as a note list and may be used with per-note modifier lists of the same length.
- `seq` currently requires explicit octaves in note literals.

## Modifier Lines

Modifier lines adjust the immediately preceding pattern line.

```loom
C4 | ^ ^ ^ ^ |
v  | !80 60 . |
p  | +2      |
```

Kinds:

- `v` velocity
- `p` pitch shift

Value rules:

- `v` is absolute velocity with default `100`
- `p` is relative semitone offset with default `0`
- `!` latches the value so later empty slots reuse it
- `.` means explicit empty slot
- Scalar values broadcast across a group
- `[...]` aligns modifier values with nested pattern structure
- `100,80` style note lists are supported for `seq` chord tokens and must match chord size

Example with a group:

```loom
hh | ^ ^ ^ [^ ^] |
v  | 90  . 80  [70 60] |
```

Behavior to remember:

- The compiler flattens pattern leaves in depth-first order when resolving modifiers.
- A group-valued modifier can target nested leaves one-for-one.
- Modifier lines only apply to the immediately preceding pattern line.

## Track Init Lines

Track init lines emit setup events at time `0`.

```loom
## pc 4
## sound 81
## bank 0/32
## cc 11 100
## pan 64
## volume 90
## expression 110
## mod 64
## sustain 127
```

Supported init commands:

- `pc <0..127>`: Program Change
- `sound <0..127>`: alias of `pc`
- `bank <msb>/<lsb>`: Bank Select using CC0 and CC32
- `cc <controller 0..127> <value 0..127>`: arbitrary Control Change
- `pan <0..127>`: CC10
- `volume <0..127>`: CC7
- `expression <0..127>`: CC11
- `mod <0..127>`: CC1
- `sustain <0..127>`: CC64

Notes:

- Commands are parsed case-insensitively.
- Formatting normalizes the canonical whitespace, for example `## pc 4`.
- Duplicate program changes on a track are rejected.
- `bank` and `cc 0` conflict with each other.

## Templates

Templates define reusable sequences.

Definition:

```loom
# @riff
C4,E4,G4 | ^ |
```

Calls:

```loom
# Lead: 1
[@riff]
[@riff +12]
[@riff x2 /2]
[@riff vel:80 pan:32]
```

Supported template parameters:

- `+N` / `-N`: transpose
- `xN`: structural repeat
- `/N`: time scale
- `rev`
- `arp`
- `strum`
- `vel:N`
- `pan:N`
- `*N`: repeat the whole expansion

Template rules:

- Multiple template calls on the same line are processed sequentially.
- Template definitions can reference other templates.
- Circular references are rejected during compilation.
- Template names use alphanumeric, `-`, and `_` characters.
- Template calls can appear in track bodies.

Example:

```loom
# Lead: 1
[@riff +12][@riff]

# @riff
C4 | ^ . ^ . |
```

## Formatting

The formatter normalizes source without changing semantics.

Canonical forms:

- Track header: `# Name: 1 s x`
- Track init: `## pc 4`
- Template calls on the same line: `[@a] [@b]`

Layout rules:

- Metadata-like lines are separated from surrounding music blocks with one blank line.
- Consecutive blank lines are squeezed to one.
- Trailing spaces are removed.
- Files end with a newline.

Treat formatter output as the canonical text form when writing or rewriting Loom.

## Semantics

Keep these in mind when generating or editing files:

- Solo filtering happens before compilation.
- Muted tracks are always excluded.
- `track.channel` is 1-based in source and 0-based in emitted MIDI.
- `pitch` transposes the whole song before compilation.
- `humanize` is deterministic for a given seed.
- `loop_range` is half-open.
- `swing` is optional and off by default.

## Quick Examples

Simple melody:

```loom
---
bpm: 100
signature: 4/4
unit: bar
---

# Piano: 1

C3 | ^ . . . | . . . . |
D3 | . ^ . . | . . . . |
E3 | . . ^ . | . . . . |
F3 | . . . ^ | . . . . |
```

Chord and modifier example:

```loom
---
bpm: 120
---

# Lead: 1

C4,E4,G4 | ^ . ^ . |
v        | 100 . 80 . |
p        | +0  . +12 . |
```

Template example:

```loom
---
bpm: 120
---

# @riff
C4,E4,G4 | ^ |

# Piano: 1
[@riff]
[@riff +12]
```

## What To Prefer When Generating Loom

- Use canonical spacing and let the formatter handle layout.
- Prefer explicit octaves in note literals.
- Use lower-case `seq`, `v`, `p`, `pc`, `sound`, `bank`, `cc`, `pan`, `volume`, `expression`, `mod`, `sustain`.
- Keep chord literals compact, for example `C4,E4,G4`.
- Use `>` for comments, not `#`.
- Use `---` for track wrap, not for repeat bars.
- Use `s` and `x` only on track headers.

## Common Pitfalls

- `seq` chord literals cannot contain spaces inside the literal.
- `loop_range` is not inclusive at the end.
- `swing: 0` disables swing; positive values must be powers of two when numeric.
- `humanize.timing` must be finite and non-negative.
- `humanize.velocity` must be `0..127`.
- Template structural repeat and repeat multiplier must be greater than zero.
- Invalid track init commands are rejected.
- Invalid notes, pitches, and out-of-range velocities are rejected at parse or compile time.

## Compact Symbol Reference

- `#`: track header / template header prefix
- `@`: template prefix inside `# @name` and `[@name]`
- `>`: comment prefix
- `---`: track wrap / frontmatter boundary
- `|`, `|:`, `:|`, `:|:`: bar markers
- `^`: note onset
- `.`: rest
- `-`: sustain
- `[` `]`: group delimiters
- `!`: modifier latch
- `+` / `-`: transpose or signed values

This document should be enough for an agent to read, edit, and generate valid Loom without consulting anything else.
