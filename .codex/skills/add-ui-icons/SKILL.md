---
name: add-ui-icons
description: Add, replace, or review icons in loom's frontend UI. Use when working on the Playground app to iconize controls, choose or extend icon usage, replace hand-written SVGs, ensure tooltips/ARIA labels for icon-only buttons, or keep icons consistent with Oat, Preact, and lucide-preact.
---

# Add UI Icons

Use this for loom's Playground UI under `apps/playground`. Prefer Oat primitives and `lucide-preact` over hand-written SVG paths.

## Workflow

1. Inspect existing usage:
   - Check `apps/playground/package.json` for icon dependencies.
   - Search `apps/playground/src` for `lucide-preact`, `IconButton`, `data-tooltip`, and `.icon`.
   - Reuse existing component patterns before adding new abstractions.

2. Choose icons:
   - Use `lucide-preact` for general UI icons.
   - Import from the package root; do not use unsupported internal package paths.
   - Prefer icons that match the command literally enough to understand with a tooltip.
   - Avoid hand-written SVG paths unless the icon is loom-specific and not available in Lucide.

3. Wire icon-only controls:
   - Use Oat's `.icon.small` class for compact icon buttons unless local UI requires otherwise.
   - Add `aria-label` and `data-tooltip` to every icon-only button.
   - Mark decorative icons with `aria-hidden="true"`.
   - Keep visible text for ambiguous or high-risk commands when an icon plus tooltip is not enough.

4. Keep app vocabulary stable:
   - Use a small `IconButton` adapter when several buttons share the same accessibility and tooltip behavior.
   - Use a typed icon map when app terms differ from Lucide names, such as `compile`, `fragment`, or `entry`.
   - Remove obsolete hand-written `iconPath` maps and icon-specific CSS after replacing them.

5. Validate:
   - Run `just playground::test`.
   - Run `just playground::app-build`.
   - Run `just ci` when code changed.

## Pattern

```tsx
import { Play, Square, type LucideIcon } from "lucide-preact";

function IconButton({
  icon: Icon,
  label,
  disabled,
  onClick,
}: {
  icon: LucideIcon;
  label: string;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      class="icon small"
      aria-label={label}
      data-tooltip={label}
      disabled={disabled}
      onClick={onClick}
    >
      <Icon aria-hidden="true" size={16} strokeWidth={2} />
    </button>
  );
}

const icons = {
  play: Play,
  stop: Square,
} satisfies Record<IconName, LucideIcon>;
```
