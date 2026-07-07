# Design

Loom interfaces should feel text-first, quiet, and tool-like. Documentation, Playground, Studio, and future user-facing surfaces should look related even when they are implemented with different UI technologies.

This document is the source of truth for shared visual guidance. Each surface maps these choices into its own implementation:

- site: VitePress theme variables such as `--vp-*`
- Playground: Oat variables and small app CSS
- Studio: terminal UI styles
- future native or plugin surfaces: native theme values

## Principles

- Keep the source text primary.
- Prefer restrained, dense tool layouts over marketing-page presentation.
- Avoid decorative gradients, heavy cards, and visual effects that compete with the editor.
- Use consistent color roles across surfaces, even when exact implementation variables differ.
- Let each surface adapt the guidance to its medium instead of sharing web-specific CSS everywhere.

## Color Roles

| Role | Usage |
| --- | --- |
| `background` | Page or app background |
| `background-soft` | Side panels, muted sections, inactive surfaces |
| `text` | Primary text |
| `text-muted` | Secondary text, metadata, inactive labels |
| `border` | Borders and dividers |
| `brand` | Links, focus, primary actions |
| `danger` | Destructive actions and errors |
| `warning` | Warnings and recoverable problems |
| `success` | Successful compile or playback state |

## Shape

- Use radius `8px` or less for panels and controls unless a surface has a stronger native convention.
- Avoid nested cards.
- Prefer clear borders and spacing over shadows.
- Keep tool panels compact and scannable.

## Typography

- Use a readable sans-serif for interface text.
- Use a readable monospace font for source, code, diagnostics, and event details.
- Editor readability is more important than brand expression.

## Brand Assets

- Keep the source app icon at `assets/brand/icon-1024.png`.
- Treat public favicons and app icons as derived assets for each surface.
- Keep derived icons visually consistent across site and Playground unless a surface has a specific technical constraint.

## Playground Guidance

- The first viewport should be the working tool: file tree, editor, diagnostics, and transport.
- The design should feel like part of the docs site, but with the density needed for editing.
- Oat or other UI helpers may be used to reduce hand-written CSS, but the shared design language remains this document rather than a CSS package.
