# Use local template libraries before registries

Loom will support template libraries as local `.loom` files referenced from the `templates` frontmatter key. Each entry maps a template library alias to a string path. A template library may also use `templates` to reference other local template libraries, but library aliases are file-local and are not re-exported.

We choose local paths first because template reuse should work without introducing package resolution, registries, versions, lockfiles, or publishing workflows. The frontmatter shape leaves room to add richer template library sources later, but registry-like sources are intentionally out of scope for now.

**Considered Options**

- Local path only: simple and explicit, but does not support shared remote distribution.
- Registry-backed libraries: useful for sharing, but requires dependency resolution, versioning, trust, caching, and lockfile decisions.
- Textual include: simple, but blurs the boundary between song fragments and template reuse.

**Consequences**

Template library source resolution is initially filesystem-based and relative to the file that declares the alias. Libraries may depend on other libraries through their own `templates` frontmatter, so cycle detection is required. Future registry support should extend the template library source model instead of changing template call syntax.
