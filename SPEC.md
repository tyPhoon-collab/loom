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

Default is `equal`

All formatters align by bar in the track, but differ in their token spacing.

> [!IMPORTANT]
> Formatters **MUST NOT** change the semantics of the score.
> - Missing blocks are **NOT** automatically padded. If a track has fewer blocks than others, it remains shorter.
> - The number of bars and blocks is preserved exactly as input.

### Minimize

- Separates tokens with a single space.
- Left-aligned.
- Compact and simple implementation.

```
# Track: 1
F4,C5 |           | ^ ^ ^ ^ [^ ^ [^ ^ ^]] |
G4,B4 | ^ ^ ^ ^ ^ |                       |
G3    | . . ^     |                       |
F3    |           | ^ - -                 |
E3    | . ^ .     | . ^ .                 |
C3    | ^ - -     |                       |
B2    |           | . . ^                 |
A2    |           | ^ ^                   |
G2    | ^ ^       |                       |
F1    |           | ^                     |
C1    | ^         |                       |
```

### Justify

- Distributes tokens with equal spacing within each block (Character-Level Justification).
- Left-aligned.
- Designed for readability while maintaining simplicity.

```
# Track: 1
F4,C5 |           | ^ ^ ^ ^ [^ ^ [^ ^ ^]] |
G4,B4 | ^ ^ ^ ^ ^ |                       |
G3    | .   .   ^ |                       |
F3    |           | ^         -         - |
E3    | .   ^   . | .         ^         . |
C3    | ^   -   - |                       |
B2    |           | .         .         ^ |
A2    |           | ^                   ^ |
G2    | ^       ^ |                       |
F1    |           | ^                     |
C1    | ^         |                       |
```

### Equal

- Distributes tokens based on a fixed grid of slots (determined by the maximum number of tokens in any block in the column).
- Tokens are assigned to slots based on their index ratio.
- Ensures vertical alignment of "beats" when token counts match grid size.
- Useful for structured drum patterns where each "slot" represents a 16th note or similar.

```
# Track: 1
F4,C5 |           | ^ ^ ^ ^ [^ ^ [^ ^ ^]] |
G4,B4 | ^ ^ ^ ^ ^ |                       |
G3    | .   .   ^ |                       |
F3    |           | ^   -   -             |
E3    | .   ^   . | .   ^   .             |
C3    | ^   -   - |                       |
B2    |           | .   .   ^             |
A2    |           | ^       ^             |
G2    | ^       ^ |                       |
F1    |           | ^                     |
C1    | ^         |                       |
```

### Time

- Positions tokens based on the Least Common Multiple (LCM) of token counts in the block column.
- Simulates a linear time axis (Piano Roll).
- Guarantees correct relative timing visualization for polyrhythms (e.g., 2 against 3).
- May result in wider blocks due to LCM grid resolution.

```
# Track: 1
F4,C5 |                                                             | ^           ^           ^           ^           [^ ^ [^ ^ ^]]           |
G4,B4 | ^           ^           ^           ^           ^           |                                                                         |
G3    | .                   .                   ^                   |                                                                         |
F3    |                                                             | ^                   -                   -                               |
E3    | .                   ^                   .                   | .                   ^                   .                               |
C3    | ^                   -                   -                   |                                                                         |
B2    |                                                             | .                   .                   ^                               |
A2    |                                                             | ^                             ^                                         |
G2    | ^                             ^                             |                                                                         |
F1    |                                                             | ^                                                                       |
C1    | ^                                                           |                                                                         |
```
