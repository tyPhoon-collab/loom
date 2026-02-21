# Loom DSL Specification

## Top Level
```ebnf
Song        = [ Frontmatter ] { Track } ;
Frontmatter = "---" , newline , yaml_content , "---" , newline ;
Track       = TrackHeader , { Line } ;
TrackHeader = "#" , space , name , ":" , space , channel , newline ;
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
RowHeader   = NoteList | DrumName ;
NoteList    = NoteName , { "," , NoteName } ;
NoteName    = ( "a"..."g" | "A"..."G" ) , [ "#" | "b" ] , digit ;
DrumName    = "bd" | "kick" | "sn" | "snare" | "rs" | "rim" | "cp" | "clap"
            | "hh" | "hc" | "hihat" | "oh" | "ho" | "hp"
            | "cr" | "crash" | "rd" | "ride" | "splash" | "china"
            | "ht" | "mt" | "lt" | "ft" | "cb" | "tamb" ;

> [!NOTE]
> - **NoteName** is case-insensitive (e.g., `c4` and `C4` are the same).
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
ModifierLine  = ModifierKind , space , Bar , { ModifierValue | space } , { Bar , { ModifierValue | space } } , Bar ;
ModifierKind  = "v" | "p" ;
ModifierValue = [ "!" ] , [ "+" | "-" ] , digits ;
```

- **`v`** (Velocity): Absolute value (0–127). Default: 100.
- **`p`** (Pitch): Relative semitone offset (`+N` / `-N`). Default: 0.
- **`!` prefix (Latch)**: The value persists for subsequent empty slots.
- **Empty slot**: Uses the latched value if active, otherwise the default.

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

## Lexical Rules
- `newline`: Line ending (`\n`, `\r\n`).
- `space`: Horizontal whitespace.
- `yaml_content`: Valid YAML string.
- `name`: Track name (string).
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

