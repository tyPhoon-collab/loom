# Song Manifests and Fragments Draft

This document is a draft for an unimplemented language feature. Delete this draft once the accepted parts are implemented and incorporated into the language specification.

## Goals

- Let large songs be split into song structure blocks such as intro, verse, chorus, and outro.
- Keep the song manifest as the source of truth for song metadata, track identity, track init, and arrangement.
- Keep song fragments focused on musical content.
- Avoid textual include semantics that make track ownership and fragment boundaries ambiguous.
- Preserve the existing single-file song style for small songs.

## Terms

- **Song Manifest**: A top-level song form that defines frontmatter, tracks, track init, and fragment calls.
- **Song Fragment**: A reusable portion of a song that is not complete by itself.
- **Fragment Call**: A wikilink-like reference that places a song fragment into a song.
- **Track Reference**: A fragment-local reference to a manifest-defined track.

## File Roles

Small songs may remain single `.loom` files with the existing syntax.

Large songs may use a song manifest plus song fragments. Both manifests and fragments use the `.loom` extension, but fragments are evaluated through a manifest context and are not standalone songs.

## Manifest Syntax

A song manifest is detected when a top-level file contains one or more fragment calls.

Fragment calls use wikilink-like syntax and must appear alone on a line:

```loom
[[intro]]
[[verse-a]]
[[chorus]]
```

The manifest may contain:

- Frontmatter
- Track headers
- Track init lines
- Fragment calls
- Comments
- Blank lines

The manifest must not contain:

- Pattern lanes
- `seq` lines
- Modifier lines
- Template definitions
- Template calls

Track channels in a manifest that uses fragments must be unique.

Fragment paths are explicitly mapped in frontmatter. The fragment call name is a fragment identity, not a filesystem path.

```loom
---
title: Demo
bpm: 120
fragments:
  intro: sections/intro.loom
  verse-a: sections/verse-a.loom
  chorus: sections/chorus.loom
---

# Piano: 1
## pc 4
## pan 64

# Bass: 2
## pc 33

# Drums: 10

[[intro]]
[[verse-a]]
[[chorus]]
```

Fragment paths are resolved relative to the manifest. Exact path safety rules are deferred, but absolute paths and parent traversal should be considered carefully before being allowed.

## Fragment Syntax

A song fragment may contain:

- Track references
- Pattern lanes
- `seq` lines
- Modifier lines
- Template definitions
- Template calls
- Track wraps
- Comments
- Blank lines

A song fragment must not contain:

- Frontmatter
- Track headers
- Track init lines
- Track solo or mute flags
- Fragment calls

Track references use a channel number:

```loom
# 1
C4 | ^ . ^ . |

# 10
kick  | ^ . . . |
snare | . . ^ . |
```

`# 1` references the manifest-defined track on MIDI channel 1. It does not define a new track.

Each track reference may appear at most once per fragment. Multiple lanes for the same track should be grouped under that track reference. Use `---` track wrap inside the track reference when the same track needs to continue later within the fragment.

Track references may be empty. An empty track reference emits no events and does not affect fragment length.

Track reference order is not required to match manifest track order.

## Template Scope

Templates defined in a fragment are local to that fragment.

Different fragments may define templates with the same name without conflict. A fragment may call only templates visible in its own fragment scope, plus any future template library mechanism explicitly added to the language.

Manifest-local template definitions are not allowed. Shared templates should be handled by a future template library mechanism, likely configured through frontmatter rather than embedded in the manifest.

## Resolution

Fragment calls are resolved through the manifest frontmatter mapping.

- A fragment call with no mapping is an error.
- A mapped fragment path that cannot be read is an error.
- A fragment track reference whose channel is not defined in the manifest is an error.
- Duplicate manifest track channels are an error when fragments are used.
- Duplicate track references within a single fragment are an error.

Fragment names should initially be restricted to ASCII letters, digits, `_`, and `-`, with an alphanumeric first character.

## Playback Semantics

Fragment calls are played in manifest order.

Each fragment call forms one song structure block. The next fragment call starts after the previous fragment's length.

The length of a fragment is the maximum length of its track references after track wrap expansion. Tracks not referenced in a fragment are silent for that fragment. Empty track references do not affect length.

Repeated calls to the same fragment are allowed and are evaluated independently.

## Deferred Decisions

- Exact frontmatter schema for fragment path mappings.
- Path safety rules for fragment mapping values.
- CLI behavior for directly checking or playing a fragment file.
- Whether diagnostics should suggest similarly named fragments when a call is unresolved.
- Whether and how to add a template library mechanism.
