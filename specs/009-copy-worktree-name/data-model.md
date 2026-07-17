# Phase 1 Data Model: Copy Worktree Name to Clipboard

This feature adds no persisted state and no new struct/entity. It adds one enum variant to each
of two existing, already-tested enums, and reads an existing derived value. Nothing here is
persisted; the persisted `State` (Clone/Eq) is unchanged.

## `Icon::Copy` (in `src/icons.rs`)

An addition to the existing, closed `Icon` vocabulary (Principle VIII: one icon per concept,
compile-time-checked — referencing a nonexistent icon is a compile error).

| Field | Value | Meaning |
|-------|-------|---------|
| Variant | `Icon::Copy` | The copy-to-clipboard action, used wherever a "Copy" affordance appears. |
| Glyph | `U+E14D` (`content_copy`) | Pinned codepoint, regression-locked by `tests/icons.rs` and verified present in the shipped font by `tests/icons_font.rs`. |

## `Message::TextCopyRequested(String)` (in `src/app.rs`)

A generic, deliberately reusable message (see research.md R5) — not modeled as a
worktree-specific type, since the same shape can serve any future read-only label.

| Field | Type | Meaning |
|-------|------|---------|
| payload | `String` | The exact text to place on the system clipboard — already resolved by the caller (e.g. the worktree's displayed name, honoring any rename override) before the message is dispatched. |

Contract (full detail in `contracts/clipboard-copy.md`):
- The pure core reducer treats this message as a no-op (alongside its sibling
  `TerminalCopyRequested`/`TerminalPasteRequested`) — clipboard I/O is a binary/GUI-runtime
  concern, not core state.
- The binary's handler writes `payload` to the system clipboard via `iced::clipboard::write`,
  and additionally dismisses the worktree context menu (documented limitation, research.md R5).

## Existing value read, not introduced, by this feature

### Worktree displayed name (`State::worktree_display_name`, in `src/app.rs`)

Already-existing derived value (feature 008): the custom rename override if the user has set
one, otherwise a name derived from the worktree's branch/directory. This feature reads it (via
the sidebar's existing `worktree_menu_items` call site) to populate the payload above; it
introduces no new field or derivation logic of its own.
