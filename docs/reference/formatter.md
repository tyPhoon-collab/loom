# Formatter Specification

## Core Rules

1. Bars must align vertically.
2. Left alignment only (no center alignment).
3. Formatting is local to the current block context.
4. Formatting must not change semantics.

Semantics-preserving means:

- block count is preserved
- bar structure is preserved
- token structure is preserved

## Vertical Layout (Islands of Meta)

Loom treats metadata-like lines as isolated islands to keep the score readable.

Metadata lines:

- `TrackHeader`
- `Comment`
- `TrackWrap`
- `Frontmatter`

Rules:

- Metadata lines are separated by one blank line from surrounding data blocks.
- Pattern and modifier lines in the same performance block remain adjacent.
- Consecutive blank lines are squeezed to a single blank line.
- Trailing spaces are removed.
- File ends with a newline.

## Template Expansion Formatting

Multiple template expansions in one input line are emitted as compact, independently normalized forms separated by a single space.

Example:

- input: `[@a][@b x9 +4]`
- output: `[@a] [@b x9 +4]`

## Loom Grid Algorithm

The formatter balances rhythmic alignment and horizontal compactness.

### 1) Grid Size (`G`)

For a block column with token counts `K_i` per row:

- Start with `LCM(K_i)`.
- Choose the smallest multiple of that LCM that is at least `K_max * 2`.
- Clamp `G` to `24`.
- Ensure `G >= K_max`.

### 2) Placement

Each block has `G + 2` columns in layout space.

- index `0` is leading padding
- token `j` of a row with `K` tokens is placed at:

`1 + floor(j * G / K)`

- trailing padding is kept before the bar

### 3) Elastic Width

Columns are elastic:

- each column width is the max token width in that column
- dense columns expand only where needed
- vertical alignment is preserved across rows

### 4) Group Handling

`[...]` is treated as one token for outer-grid calculations.

- no inner-grid alignment at formatter level
- group occupies its full required width in its slot
- prevents unnecessary outer-grid explosion
