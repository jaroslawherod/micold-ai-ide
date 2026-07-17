# Phase 0 Research: Copy Worktree Name to Clipboard

All items below were resolved by reading the pinned iced 0.13 source and the app's own code; no
open NEEDS CLARIFICATION remain.

## R1 — Where is the actual gap?

**Decision**: Editable text fields need no change. iced's `text_input` widget already handles
Ctrl+C (copy), Ctrl+X (cut), Ctrl+V (paste), and Ctrl+A (select-all) natively via
`state.keyboard_modifiers.command()` (Linux/Windows: Ctrl; macOS: Cmd) — verified directly in
`iced_widget-0.13.4/src/text_input.rs` (the `Character("c")`/`"x"`/`"v"`/`"a"` match arms guarded
by `command()`, lines ~737–907 of that file). Every editable field in this app (project rename,
worktree rename, the worktree-creation form, Settings) is a plain `text_input`, so all of them
already had this for free. The real, reported gap is **read-only labels** — worktree names shown
via the plain `text` widget — which has no selection or copy behavior in iced 0.13 at all.

**Rationale**: Scoping the fix to the actual gap (read-only labels) avoids touching working
code and avoids inventing app-level copy/paste handling that would shadow or conflict with
`text_input`'s built-in behavior.

**Alternatives considered**:
- *Add explicit Ctrl+C/V handling to every text input* — rejected: would duplicate behavior
  `text_input` already provides, and risks double-handling / conflicting with it.
- *Build a custom selectable-text widget for labels* — rejected for this change (see R5): iced
  0.13 has no selection primitive to build on cheaply, and a context-menu action fully satisfies
  the reported need with far less surface area.

## R2 — Overlay dialogs already win the keyboard race against the terminal

**Decision**: No change needed to input routing. `iced_widget::stack`'s `on_event` dispatches to
children in **reverse** order (`self.children.iter_mut().rev()...find(|&status| status ==
Captured)`, `iced_widget-0.13.4/src/stack.rs`), i.e. the topmost-rendered child (a modal dialog,
via `opaque`/`Modal`) sees keyboard events before anything beneath it (the terminal pane). So a
focused `text_input` inside an open dialog already receives Ctrl+C/V before the embedded
terminal's own key routing (`src/ui/mod.rs`'s `terminal_focused` gate) ever runs.

**Rationale**: Confirms there is no dispatch-order bug to fix — the "terminal steals the
shortcut" failure mode some editors exhibit does not apply here.

## R3 — Terminal keeps Ctrl+Shift+C/V, not Ctrl+C/V

**Decision**: Leave the embedded terminal's chords unchanged (`src/keymap.rs`
`copy_paste_action`: macOS Cmd+C/V, else Ctrl+Shift+C/V) rather than unifying them with the
plain Ctrl+C/V used elsewhere.

**Rationale**: In a shell, Ctrl+C is SIGINT — remapping it to "copy" would break interrupting a
running process, a far worse regression than the inconsistency of a different chord. This is
existing, intentional behavior (feature 006), not part of this change's scope.

## R4 — Icon: which glyph, and the font-subset consequence

**Decision**: Use Material Symbols' `content_copy` glyph, codepoint `U+E14D` — confirmed present
in the upstream `MaterialSymbolsOutlined[FILL,GRAD,opsz,wght].codepoints` manifest
(`google/material-design-icons`). The app's shipped font
(`assets/fonts/MaterialSymbolsOutlined.ttf`) is a **subset** containing only the codepoints
`Icon::ALL` currently uses (`assets/fonts/PROVENANCE.md`), so adding `Icon::Copy` required
regenerating that subset with `U+E14D` added to the `pyftsubset --unicodes` list, following the
exact documented process: `varLib.instancer` (wght=400/FILL=0/GRAD=0/opsz=24) then `pyftsubset`.
`pip`/`fontTools` were not preinstalled; the regeneration used `mise exec -- uvx --from fonttools
fonttools ...` / `pyftsubset ...` (`uv` was already declared in `mise.toml`), so no new
persistent tool dependency was added to the project.

**Verification (genuine red→green)**: `tests/icons_font.rs::every_icon_codepoint_has_a_glyph`
was run against the *unmodified* font first and observed failing
("`Copy (U+E14D) has no glyph in the shipped font — would render as tofu`"), then passing once
the regenerated subset replaced the shipped `.ttf`.

**Alternatives considered**:
- *Reuse an existing glyph (e.g. `Rename`'s pencil) for Copy* — rejected: would misrepresent the
  action and violate the app's one-icon-per-concept convention (`docs/user-guide/icons.md`).
- *Ship the full, unsubsetted variable font* — rejected: contradicts the documented
  minimal-footprint convention and adds ~10MB for one glyph.

## R5 — Message shape: generic vs. worktree-specific

**Decision**: Add one generic `Message::TextCopyRequested(String)` rather than a
`WorktreeNameCopyRequested` (or similarly narrow) variant. The binary's handler writes the
payload to the clipboard via `iced::clipboard::write`, the exact mechanism
`Message::TerminalCopyRequested` already uses for the terminal's own Copy action.

**Rationale**: The same underlying gap — a read-only label with no way to copy it — exists for
other labels this change does not touch (session titles, known-project names; see spec.md
Assumptions). A generic "copy this text" message means extending the same pattern to those
labels later is a one-line context-menu addition, not a new message variant each time —
consistent with Principle VIII's reuse intent applied to the message layer, not just widgets.

**Alternatives considered**:
- *`WorktreeNameCopyRequested(String)`* — rejected: needlessly narrow given the identical need
  already visible elsewhere; would need re-deriving the same design for every future label.
- *A generic "copy" command with no payload, reading state at handle-time* — rejected: the
  binary's message handler has no reliable way to know *which* label triggered it without a
  payload; passing the already-resolved text is simpler and avoids re-deriving it.

**Known limitation, documented, not fixed**: the binary's handler for `TextCopyRequested`
unconditionally also dismisses the worktree context menu (`Message::WorktreeMenuDismissed`),
because today it is only ever triggered from that menu. This is harmless (dismissing an
already-closed menu is a no-op) but is a small coupling a future non-worktree call site should
be aware of — see `contracts/clipboard-copy.md`.
