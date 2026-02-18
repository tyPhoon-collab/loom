# Loom DSL Specification

## 1. Top Level
```ebnf
Song        = [ Frontmatter ] { Track } ;
Frontmatter = "---" , newline , yaml_content , "---" , newline ;
Track       = TrackHeader , { Line } ;
TrackHeader = "#" , space , name , ":" , space , channel , newline ;
```

## 2. Lines
```ebnf
Line        = CommentLine | PatternLine | EmptyLine ;
CommentLine = ">" , { character } , newline ;
EmptyLine   = { space } , newline ;
PatternLine = RowHeader , Bar , Block , { Bar , Block } , Bar , [ space , CommentLine ] ;
```

## 3. Patterns & Tokens
```ebnf
RowHeader   = NoteList | DrumName ;
NoteList    = NoteName , { "," , NoteName } ;
NoteName    = ( "a"..."g" | "A"..."G" ) , [ "#" | "b" ] , digit ;
DrumName    = "bd" | "kick" | "bassdrum" | "sd" | "snare" | "rim" | "rs" | "sidestick"
            | "clap" | "handclap" | "cp" | "hc" | "hihat" | "hihatclosed" | "ho"
            | "hihatopen" | "hp" | "hihatpedal" | "crash" | "ride" | "splash" | "china"
            | "ht" | "himidtom" | "mt" | "lowmidtom" | "lt" | "lowtom" | "ft"
            | "highfloortom" | "cb" | "cowbell" | "tamb" | "tambourine" ;

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

## 4. Lexical Rules
- `newline`: Line ending (`\n`, `\r\n`).
- `space`: Horizontal whitespace.
- `yaml_content`: Valid YAML string.
- `name`: Track name (string).
- `channel`: MIDI channel (1-16).
- `character`: Any UTF-8 character except newline.
- `digit`: `0`..."9".
- `alphabetic`: `a`..."z" | `A`..."Z".

## 5. Frontmatter Defaults
If frontmatter is omitted, the following defaults are used:
- `bpm`: 120
- `signature`: "4/4"
- `unit`: "bar"
- `pitch`: 0
