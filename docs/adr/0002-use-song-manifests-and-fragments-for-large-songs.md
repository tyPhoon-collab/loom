# Use song manifests and fragments for large songs

Loom will support large songs by allowing a song manifest to define song metadata, tracks, track init, and fragment calls, while song fragments hold musical content through track references. We choose this over textual include because the manifest remains the source of truth for track identity and arrangement, fragments stay focused on song structure blocks, and fragment paths can be explicitly mapped in frontmatter instead of being implied by filesystem layout.

The draft language shape is tracked in [Song Manifests and Fragments Draft](../language/manifest-fragments-draft.md). Detailed syntax and validation rules belong in the language specification once the feature is implemented; this ADR records the boundary decision between manifests, fragments, and textual include.

**Considered Options**

- Textual include: simple to implement, but it makes fragment boundaries weak and spreads track definitions across files.
- Convention-based fragment lookup: concise, but it makes filesystem layout part of the language contract.
- Manifest-mapped fragments: more explicit, but keeps fragment identity and file paths separate and makes the manifest the source of truth.

**Consequences**

Song manifests and song fragments are different top-level forms even though both use `.loom` files. A fragment is evaluated through a manifest context, where track references resolve against manifest-defined tracks and fragment calls resolve through the manifest frontmatter mapping.
