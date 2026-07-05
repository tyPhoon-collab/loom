# Concept

Loom is a text-first language for writing musical structure as readable score-like code.

## Core Ideas

- Markdown-like structure: headers for tracks, frontmatter for metadata.
- Sparse piano-roll style: write only required rows.
- Elastic grid: timing is inferred by equal subdivision within bars.
- Nested cycle: `[...]` enables recursive subdivision.
- Stateful sustain: ties can continue across block boundaries.

## Syntax Mental Model

- `^`: trigger note
- `.`: rest
- `-`: sustain previous note
- `|`: time boundary
- `[...]`: nested subdivision

Whitespace is visual and non-semantic.
