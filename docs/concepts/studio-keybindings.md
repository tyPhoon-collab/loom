# Studio Keybinding Policy

This page records the current direction for Studio keybinding design.

## Status

Accepted on 2026-06-15.

## Decision

Studio should prefer a Vim-like mental model where it fits the product.

In particular:

- Prefer operator-style commands over unrelated mnemonic shortcuts.
- Reserve `d` for delete-oriented behavior and delete prefixes.
- Keep `x` as a direct small-scope delete when it improves speed.
- Treat duplication as distinct from delete.
- Move duplication toward a yank / paste style model (`y` / `p`) instead of keeping it on `d`.

## Rationale

The old `D` / `d` split made users translate between two meanings:

- `D` meant delete prefix
- `d` meant duplicate

That was harder to remember than a model where `d` consistently means delete.

Vim already gives many users a strong mental model:

- `d` is delete
- `x` is a direct delete
- duplication is usually expressed as copy/yank plus paste, not as a standalone duplicate command

Reusing that model should reduce keybinding surprise in Studio.

## Consequences

- Future Studio keybinding changes should move toward `d` as the main delete operator.
- Studio should use `y` / `p` semantics for copy-like edits instead of a dedicated duplicate key.
- Dedicated duplicate bindings should not return unless `y` / `p` proves insufficient.
- Help text and guide pages should stay aligned with whichever bindings are currently implemented.
