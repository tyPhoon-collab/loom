# Loom

Loom is the language and product context for text-first MIDI composition, playback, and live coding. This glossary defines the domain language shared by the DSL, compiler, runtime tools, documentation, and Studio.

## Language

**Song**:
A complete Loom composition made of frontmatter and one or more tracks.
_Avoid_: Composition, document, file

**Song Manifest**:
A top-level song form that defines song metadata, tracks, and fragment calls.
_Avoid_: Config file, parent file

**Song Fragment**:
A reusable portion of a song that is not a complete song by itself.
_Avoid_: Partial song, file, part

**Fragment Call**:
A reference that places a song fragment into a song.
_Avoid_: File call, include, import

**Frontmatter**:
The song-level metadata and settings written before the tracks.
_Avoid_: Header, config, preamble

**Track**:
A named musical part in a song, associated with a MIDI channel.
_Avoid_: Part, voice

**Track Reference**:
A fragment-local reference to a track defined by the song.
_Avoid_: Track header, channel header

**Track Init**:
A track-level setting that prepares MIDI state before the track's musical events play.
_Avoid_: Init line, track config

**Track Wrap**:
A marker that continues the current track later in the song so long patterns can be written across multiple sections.
_Avoid_: Track continuation, section break

**Pattern**:
A grid-based musical phrase in a track that describes onsets, rests, and sustains over time.
_Avoid_: Sequence, phrase, row

**Seq**:
A shorthand pattern form where note literals are written directly in the time grid.
_Avoid_: Sequence

**Lane**:
A pattern row inside a track for a single note, drum, chord, or MIDI note number.
_Avoid_: Row header, note-head line

**Lane Head**:
The left-side name of a lane that identifies the note, drum, chord, or MIDI note number controlled by that lane.
_Avoid_: Row header, note head

**Bar**:
A musical measure used as the primary visual and timing division in Loom patterns.
_Avoid_: Measure, block

**Unit**:
An editable grid position in a pattern or seq line, used by Studio navigation and structured editing.
_Avoid_: Cell, step

**Token**:
The smallest written musical unit inside a pattern or seq grid, such as an onset, rest, sustain, group, or note literal.
_Avoid_: Cell, step, symbol

**Group**:
A bracketed subdivision inside a pattern or seq grid that fits multiple tokens into one outer unit.
_Avoid_: Tuplet, bracket

**Onset**:
A token that starts a note or drum hit at its position in the grid.
_Avoid_: Hit, trigger, note

**Modifier**:
A supporting line that changes how the preceding pattern is performed.
_Avoid_: Automation, parameter, control

**Template**:
A named reusable pattern fragment that can be called from a track.
_Avoid_: Macro, snippet, preset

**Template Call**:
A use of a template inside a track, optionally with parameters that transform the reused pattern fragment.
_Avoid_: Template expansion, macro call

**Template Parameter**:
A value inside a template call that changes how the called template is reused.
_Avoid_: Template argument, macro

**Event**:
A compiled musical action produced from a song for playback or MIDI export.
_Avoid_: Message, output, command

**Live Coding**:
A composition workflow where changes to a song are reflected during playback.
_Avoid_: Hot reload, live mode, watch mode

**Studio**:
The integrated TUI editor for editing and playing Loom songs.
_Avoid_: Editor, interface

**Structured Editing**:
Editing that operates on Loom concepts such as units, bars, tracks, and template calls instead of only raw text ranges.
_Avoid_: Structure-aware editing, smart editing, semantic editing

**Selection**:
A Studio editing target that identifies one or more Loom structures for an operation.
_Avoid_: Range, region, highlight
