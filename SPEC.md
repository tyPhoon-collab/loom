# Loom DSL Specification (v0.1)

Loom DSLの構文をEBNF（Extended Backus-Naur Form）で定義します。

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
PatternLine = RowHeader , { space } , "|" , Block , { "|" , Block } , "|" , [ space , CommentLine ] ;
```

## 3. Patterns & Tokens
```ebnf
RowHeader   = NoteName | DrumName ;
NoteName    = ( "a" | "b" | "c" | "d" | "e" | "f" | "g" ) , [ "#" | "b" ] , digit ;
DrumName    = { alphabetic } ;

Block       = { space | Token } ;
Token       = NoteOn | Rest | Sustain | Group ;
NoteOn      = "^" ;
Rest        = "." ;
Sustain     = "-" ;
Group       = "[" , { space | Token } , "]" ;
```

## 4. Lexical Rules
- `newline`: Line ending (`\n`, `\r\n`).
- `space`: Horizontal whitespace.
- `yaml_content`: Valid YAML string.
- `name`: Track name (string).
- `channel`: MIDI channel (1-16).
- `character`: Any UTF-8 character except newline.
