import type { PlaygroundFile } from "../workspace/types";

export type PlaygroundExample = {
  id: string;
  name: string;
  description: string;
  entryPath: string;
  activePath: string;
  files: PlaygroundFile[];
};

export const examples: PlaygroundExample[] = [
  {
    id: "minimal-melody",
    name: "Minimal Melody",
    description: "A small single-file melody for first edits.",
    entryPath: "song.loom",
    activePath: "song.loom",
    files: [
      {
        path: "song.loom",
        content: `---
bpm: 100
signature: 4/4
unit: bar
title: "Simple Melody"
---

# Piano: 1

> Simple C Major Scale
> One note per beat (Quarter notes)

c3 | ^ . . . | . . . . |
d3 | . ^ . . | . . . . |
e3 | . . ^ . | . . . . |
f3 | . . . ^ | . . . . |
g3 | . . . . | ^ . . . |
a3 | . . . . | . ^ . . |
b3 | . . . . | . . ^ . |
c4 | . . . . | . . . ^ |
`,
      },
    ],
  },
  {
    id: "manifest-fragments",
    name: "Manifest Fragments",
    description: "A manifest-driven song assembled from section files.",
    entryPath: "song.loom",
    activePath: "song.loom",
    files: [
      {
        path: "song.loom",
        content: `---
bpm: 118
title: "Manifest Fragments"
fragments:
  intro: sections/intro.loom
  verse: sections/verse.loom
  chorus: sections/chorus.loom
---

# Lead: 1
## pc 81
## pan 72

# Bass: 2
## pc 34

# Drums: 10

[[intro]]
[[verse]]
[[chorus]]
`,
      },
      {
        path: "sections/intro.loom",
        content: `# @lead-hit
C4,E4 | ^ . . . |

# 1
[@lead-hit]

# 10
kick  | ^ . . . |
snare | . . ^ . |
hh    | ^ ^ ^ ^ |
`,
      },
      {
        path: "sections/verse.loom",
        content: `# 1
C4 | ^ . ^ . |
E4 | . ^ . ^ |

# 2
C2 | ^ . ^ . |

# 10
kick  | ^ . . ^ |
snare | . . ^ . |
hh    | ^ ^ ^ ^ |
`,
      },
      {
        path: "sections/chorus.loom",
        content: `# @lead-chord
C4,E4,G4 | ^ . ^ . |

# 1
[@lead-chord]

# 2
C2 | ^ ^ ^ ^ |

# 10
kick  | ^ . ^ . |
snare | . ^ . ^ |
hh    | ^ ^ ^ ^ |
`,
      },
    ],
  },
  {
    id: "template-library",
    name: "Template Library",
    description: "A song that calls templates from library files.",
    entryPath: "song.loom",
    activePath: "song.loom",
    files: [
      {
        path: "song.loom",
        content: `---
bpm: 120
title: "Template Library"
templates:
  drums: libraries/drums.loom
  bass: libraries/bass.loom
---

# Bass: 2
## pc 34
[@bass.root]

# Drums: 10
[@drums.kick-snare]
`,
      },
      {
        path: "libraries/common.loom",
        content: "# @kick\nkick | ^ . ^ . |\n",
      },
      {
        path: "libraries/bass.loom",
        content: "# @root\nC2 | ^ . . . |\n",
      },
      {
        path: "libraries/drums.loom",
        content: `---
templates:
  common: common.loom
---

# @kick-snare
[@common.kick]
snare | . . ^ . |
`,
      },
    ],
  },
];

export function cloneExample(example: PlaygroundExample): PlaygroundExample {
  return {
    ...example,
    files: example.files.map((file) => ({ ...file })),
  };
}
