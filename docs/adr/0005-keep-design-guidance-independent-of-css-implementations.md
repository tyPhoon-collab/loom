# Keep design guidance independent of CSS implementations

Loom will keep shared visual guidance in `DESIGN.md` instead of making VitePress CSS variables, Playground CSS, or a shared CSS file the source of truth. We choose a human-readable design document because Loom has multiple user-facing surfaces with different styling systems: docs use VitePress `--vp-*` variables, Playground may use Oat and app CSS, Studio uses terminal UI styles, and future native or plugin surfaces such as VSTs may not use CSS at all. Each surface should map the shared design guidance into its own implementation rather than depending on one web-specific token file.

**Considered Options**

- VitePress `--vp-*` variables as source of truth: fits docs, but makes docs theme internals the owner of Loom's visual language.
- Shared CSS or token file as source of truth: works for web surfaces, but does not naturally cover terminal, native, or plugin UIs.
- `DESIGN.md` as source of truth: lightweight and implementation-neutral, but requires manual mapping into each surface.
