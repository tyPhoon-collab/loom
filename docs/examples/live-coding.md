# Live Coding Examples

Examples focused on templates, modifiers, repeats, and real-time iteration.

<!-- AUTO-GENERATED:EXAMPLES-LIVE:START -->

## Live Coding (Auto)



#### Index

- `examples/live-coding/feature-init-events.loom`
- `examples/live-coding/feature-loop-basic.loom`
- `examples/live-coding/feature-loop-range.loom`
- `examples/live-coding/feature-macros.loom`
- `examples/live-coding/feature-modifier-group.loom`
- `examples/live-coding/feature-modifier.loom`
- `examples/live-coding/feature-multi-note.loom`
- `examples/live-coding/feature-mute.loom`
- `examples/live-coding/feature-nested-tracks.loom`
- `examples/live-coding/feature-pan.loom`
- `examples/live-coding/feature-pitch-shift.loom`
- `examples/live-coding/feature-repeat-complex.loom`
- `examples/live-coding/feature-repeat.loom`
- `examples/live-coding/feature-sustain.loom`
- `examples/live-coding/feature-swing-16.loom`
- `examples/live-coding/feature-swing-8.loom`
- `examples/live-coding/feature-swing-amount.loom`
- `examples/live-coding/feature-template-holizontal.loom`
- `examples/live-coding/feature-template-vertical.loom`
- `examples/live-coding/feature-track-wrap.loom`

#### Samples

##### `examples/live-coding/feature-init-events.loom`

````loom
---
bpm: 120
signature: 4/4
unit: bar
title: "Init Events Demo"
---

# Lead: 1

> canonical form
## bank 0/32
## pc 81
## cc 11 100

C4 | ^ . ^ . |
E4 | . ^ . ^ |

# Pad: 2

> sugar aliases
## volume 90
## pan 72
## expression 110

C3,G3 | ^   ^   |
````

##### `examples/live-coding/feature-loop-basic.loom`

````loom
---
bpm: 120
title: "Loop"
loop: true
---

# Piano: 1

c3 | ^ ^ ^ ^ | . . . . |
e3 | . . . . | ^ ^ ^ ^ |
````

##### `examples/live-coding/feature-loop-range.loom`

````loom
---
bpm: 120
title: "Loop Range"
loop: true
unit: bar
loop_range: 0..1
---

# Piano: 1

> This should loop ONLY the first bar indefinitely.
> Range is half-open: 0..1 means from bar 0 to before bar 1.

c3 | ^ ^ ^ ^ | . . . . |
e3 | . . . . | ^ ^ ^ ^ |
````

##### `examples/live-coding/feature-macros.loom`

````loom
# Track: 1

[@chord strum]
[@chord arp]
[@chord vel:60]

# @chord

G3 | ^ |
E3 | ^ |
C3 | ^ |
````

##### `examples/live-coding/feature-modifier-group.loom`

````loom
---
bpm: 120
signature: 4/4
unit: bar
title: "Modifier Group Demo"
---

# Drums: 10

> Demo assigning velocity to individual sub-tokens inside a Group.
> \".\" means explicit default (Empty). In [. . . 80], only the 4th gets vel=80.

hh | ^  ^  ^           ^   |
sn | .   ^   [^ . . ^]     |
v  | .   .   [. . . 80]    |
bd | ^  .  ^           .   |

# Piano: 1

> A scalar value is broadcast to the entire Group.

C3 | ^   ^ [^ ^ ^ ^]  |
v  | 100   60         |
````

##### `examples/live-coding/feature-modifier.loom`

````loom
---
bpm: 120
signature: 4/4
unit: bar
title: "Modifier Demo"
---

# Piano: 1

> Demo of velocity and pitch modifiers.
> The v line controls velocity, and the p line controls pitch shift.

C3 | ^ ^ ^ ^ |
v  | !80     |
p  | +2      |

# Drums: 10

> !127 latches the value; 60 is one-shot (then returns to default).

kick  | ^ . ^ . |
v     | !127  60 |
snare | . ^ . ^ |
````

##### `examples/live-coding/feature-multi-note.loom`

````loom
---
bpm: 120
title: "Chord Example"
---

# Piano: 1
g3,b3,d4 |     | ^ . |
f3,a3,c4 | . ^ |     |
c3,e3,g3 | ^ . | . ^ |
````

##### `examples/live-coding/feature-mute.loom`

````loom
---
bpm: 120
title: Mute Feature Example
---

# Piano: 1
C3 | ^ ^ ^ ^ | ^ ^ ^ ^ |

# Bass: 2 x
C2 | ^ . ^ . | ^ . ^ . |
> This track is muted and will not be exported to MIDI.

# Drums: 10
kick | ^ . ^ . | ^ . ^ . |
snare | . ^ . ^ | . ^ . ^ |
````

##### `examples/live-coding/feature-nested-tracks.loom`

````loom
---
bpm: 90
title: "Nested Rhythms"
---

# Percussion: 10

> Bracket [] divides the beat into N equal parts.
> [^ ^ ^] = Triplet (3 notes in 1 beat)
> [[^ ^] ^] = Nested (2 notes in half beat, then 1 note in half beat)

snare | ^ ^ ^ ^ | (4 beats)
snare | [^ ^] [^ ^] [^ ^] [^ ^] | (8th notes)
snare | [^ ^ ^] [^ ^ ^] [^ ^ ^] ^ | (Triplets + Quarter)
snare | [^ ^ ^ ^] ^ ^ ^ | (16th notes + Quarter)

> Complex Nesting
kick  | ^ . [^ [^ ^]] . |
````

##### `examples/live-coding/feature-pan.loom`

````loom
---
bpm: 120
---

# Lead: 1

> template macro pan:N emits timed CC10 before each expansion
[@riff pan:16][@riff pan:100]

# @riff
C3,E3,G3 | ^ |
````

##### `examples/live-coding/feature-pitch-shift.loom`

````loom
---
pitch: 12
---

# Melody: 1
C4 | ^ . . |

# Drums: 10
kick | ^ . . |
snare | . ^ . |
````

##### `examples/live-coding/feature-repeat-complex.loom`

````loom
> Test: Complex Mixed Repeat Notation
> This file tests multiple repeat sections mixed with standard bars.

# Melody: 1
E4 | ^ ^ |: ^ . | ^ ^ :|: ^ ^ | ^ ^ :| ^ ^ |
C4 | ^ ^ |: ^ . | ^ ^ :|: ^ ^ | ^ ^ :| ^ ^ |

# Bass: 2
G2 | ^ - |: ^ . | ^ . :| ^ - |: ^ . | ^ . :|
C2 | ^ - |: ^ . | ^ . :| ^ - |: ^ . | ^ . :|
````

##### `examples/live-coding/feature-repeat.loom`

````loom
> Test: Repeat Notation
> Format should align bars and tokens.

# Track: 1
D4 |: ^ | ^ :|
C4 |: ^ . ^ | ^ :|
````

##### `examples/live-coding/feature-sustain.loom`

````loom
---
bpm: 60
title: "Sustain Note"
---

# Bass: 2

> '-' extends the previous note.
> If '-' is at the start of a block, it extends from the previous block.

c2 | ^ - - - | - - - . | (7 beats long note)
e2 | . . ^ - | - . . . | (2 beats long, crossing bar line)

> Multiple notes sustaining independently (Polyphony in one track is just multiple rows)

g2 | ^ - ^ - | ^ - . . |
d2 | . ^ . ^ | . ^ - - |
````

##### `examples/live-coding/feature-swing-16.loom`

````loom
---
title: "Swing 16 Test"
signature: 4/4
swing: 16
loop: true
bpm: 60
---

# Track: 1

C2 | ^ ^ ^ ^  |

# Track: 2

C3 | [^ ^] [^ ^] [^ ^] [^ ^]  |

# Track: 3

C4 | [^ ^ ^ ^] [^ ^ ^ ^] [^ ^ ^ ^] [^ ^ ^ ^]  |
````

##### `examples/live-coding/feature-swing-8.loom`

````loom
---
title: "Swing Test"
signature: 4/4
swing: 8
loop: true
bpm: 60
---

# Track: 1

C2 | ^ ^ ^ ^  |

# Track: 2

C3 | [^ ^] [^ ^] [^ ^] [^ ^]  |

# Track: 3

C4 | [^ ^ ^ ^] [^ ^ ^ ^] [^ ^ ^ ^] [^ ^ ^ ^]  |
````

##### `examples/live-coding/feature-swing-amount.loom`

````loom
---
title: "Swing Amount Test"
signature: 4/4
swing:
  grid: 8
  amount: 60
---

# Track: 1
C4 | ^ ^ ^ ^ |

# Track: 2
C4 | [ ^ ^ ] [ ^ ^ ] [ ^ ^ ] [ ^ ^ ] |

# Track: 3
C4 | [ ^ ^ ^ ^ ] [ ^ ^ ^ ^ ] [ ^ ^ ^ ^ ] [ ^ ^ ^ ^ ] |
````

##### `examples/live-coding/feature-template-holizontal.loom`

````loom
# Piano: 1

[@Cmaj /2][@Fmaj /2]

# @Fmaj

[@Cmaj +5]

# @Cmaj

G3 | ^ |
E3 | ^ |
C3 | ^ |
````

##### `examples/live-coding/feature-template-vertical.loom`

````loom
---
bpm: 60
loop: true
---

# Track: 1

[@note x9 +4]
[@note x4]

# Drums: 10

[@hihat x36]

# @hihat

hh | ^ |

# @note

C3 | ^ |
````

##### `examples/live-coding/feature-track-wrap.loom`

````loom
---
bpm: 120
---

# Piano: 1
C3,G3,B3,E4 | ^ |
---
A2,G3,C4,E4 | ^ |
````

<!-- AUTO-GENERATED:EXAMPLES-LIVE:END -->
