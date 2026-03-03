# Templates

Templates are reusable sequence blocks expanded in tracks.

## Definition

```loom
# @riff
C4,E4,G4 | ^ |
```

## Expansion

```loom
# Lead: 1
[@riff]
[@riff +12]
[@riff x2 /2]
[@riff vel:80 pan:32]
```

## Supported Parameters

- `+N` / `-N`: transpose
- `xN`: structural repeat within block span
- `/N`: time scaling
- `*N`: repeat expansion sequence
- macros: `rev`, `arp`, `strum`, `vel:N`, `pan:N`

<!-- AUTO-GENERATED:TEMPLATE-MACROS:START -->

## Template Macros (Auto)

- `rev`
- `arp`
- `strum`
- `vel:N`
- `pan:N`

<!-- AUTO-GENERATED:TEMPLATE-MACROS:END -->

## Execution Rule

Multiple expansions on the same line execute sequentially.

Example:

`[@a][@b]` means `@b` starts after `@a` ends.
