# Error Reference

This page documents parser and compiler diagnostics.

## Status

WIP. Structured error catalog will be added incrementally.

## Planned Sections

- Parse errors
- Validation errors
- Compile errors
- Suggested fixes

<!-- AUTO-GENERATED:ERROR-CODES:START -->

## Diagnostic Codes

### Parser

- `loom::parser::base`
- `loom::parser::context`
- `loom::parser::frontmatter`
- `loom::parser::validation`

### Compiler

- `loom::compiler::circular_template_reference`
- `loom::compiler::context`
- `loom::compiler::invalid_channel`
- `loom::compiler::invalid_note`
- `loom::compiler::invalid_signature`
- `loom::compiler::note_out_of_range`
- `loom::compiler::template_not_found`
- `loom::compiler::velocity_out_of_range`

<!-- AUTO-GENERATED:ERROR-CODES:END -->

<!-- AUTO-GENERATED:ERROR-FIXTURES:START -->

## Error Fixtures (Auto)

### Index

- `invalid-channel.loom`
- `invalid-fenced-codeblock-modifier.loom`
- `invalid-fenced-codeblock-signature.loom`
- `invalid-frontmatter.loom`
- `invalid-init-u7.loom`
- `invalid-loop-range.loom`
- `invalid-modifier.loom`
- `invalid-signature.loom`
- `invalid-swing.loom`
- `invalid-syntax.loom`
- `invalid-template-arg-zero.loom`
- `invalid-template-cycle.loom`
- `invalid-unit.loom`
- `missing-template-nested.loom`
- `missing-template.loom`
- `note-out-of-range-high.loom`
- `note-out-of-range-low.loom`
- `velocity-out-of-range.loom`

### Samples

#### `invalid-channel.loom`

````loom
# Track: 17
C4 | ^ |
````

#### `invalid-fenced-codeblock-modifier.loom`

````loom
---
title: "Invalid Value / Modifier"
signature: 4/4
---

# Track: 1
```
C4|rev
```
````

#### `invalid-fenced-codeblock-signature.loom`

````loom
---
title: "Invalid Signature"
signature: "Invalid"
---

# Track: 1
```
C4
```
````

#### `invalid-frontmatter.loom`

````loom
---
title: "Invalid YAML"
signature: 4/4
  broken_indentation: true
---

# Track: 1
```
C4
```
````

#### `invalid-init-u7.loom`

````loom
# Track: 1
## pc 200
C4 | ^ |
````

#### `invalid-loop-range.loom`

````loom
---
loop: true
unit: "bar"
signature: "4/4"
loop_range: "0 ~ 2"
---

# Track: 1
C4 | ^ |
````

#### `invalid-modifier.loom`

````loom
# Track: 1
C4 | ^ |
v 10 |
````

#### `invalid-signature.loom`

````loom
---
title: "Invalid Signature"
signature: "Invalid"
---

# Track: 1
C4 | ^ |
````

#### `invalid-swing.loom`

````loom
---
swing:
  grid: 6
  amount: 120
---

# Track: 1
C4 | ^ |
````

#### `invalid-syntax.loom`

````loom
---
bpm: 120
title: "Invalid Syntax"
---

# Piano: 1

c3 | ^ ^ ^ |

# Invalid Track
> The following line has a syntax error (missing closing pipe or invalid char)

d3 | ^ ^ % ^ |
````

#### `invalid-template-arg-zero.loom`

````loom
# Track: 1
[@A x0]

# @A
C4 | ^ |
````

#### `invalid-template-cycle.loom`

````loom
# @A
[@B]

# @B
[@C]

# @C
[@A]

# Track: 1
[@A]
````

#### `invalid-unit.loom`

````loom
---
unit: "step"
signature: "4/4"
---

# Track: 1
C4 | ^ |
````

#### `missing-template-nested.loom`

````loom
# @A
[@B]

# @B
[@Missing]

# Track: 1
[@A]
````

#### `missing-template.loom`

````loom
---
title: "Missing Template"
signature: 4/4
---

# Track: 1
[@NonExistent]
````

#### `note-out-of-range-high.loom`

````loom
# Track: 1
B8 | ^ |
p  | +100 |
````

#### `note-out-of-range-low.loom`

````loom
# Track: 1
C0 | ^ |
p  | -100 |
````

#### `velocity-out-of-range.loom`

````loom
# Track: 1
C4 | ^ |
v  | 200 |
````

<!-- AUTO-GENERATED:ERROR-FIXTURES:END -->
