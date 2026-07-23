# Implementation Plan: Forget a Project from the Switcher's Right-Click Menu

**Branch**: `feat/make-posibile-to-remove-a-project-also-from-project-switcher` | **Date**: 2026-07-23 | **Spec**: [spec.md](./spec.md)

## Summary

Add a right-click context menu to the top-bar project switcher whose single item, **Forget
project**, dispatches feature 014's existing `Message::ProjectForgetRequested(path)`. All
forgetting logic — the confirmation, `Workspace::forget`, session teardown, persistence — is
reused unchanged; this feature contributes only the entry point and the menu's positioning.

The one genuinely new piece of logic is **where the menu is drawn**. `mouse_area::on_right_press`
carries no cursor position in iced 0.13, so the pointer is tracked in the core from a
`CursorMoved` message the binary emits, snapshotted into the open menu's anchor, and clamped at
render time against the window size so the panel can never open off-screen.

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: `iced` (GUI). **No new dependencies.**

**Storage**: None. This feature adds no persisted state — all new state is transient UI state.

**Testing**: `mise run test` (render-free core) for the pure surface: menu open/close/replace,
cursor anchoring, `clamp_menu_anchor`, popover mutual exclusion, and the hand-off into
`ProjectForgetRequested`. GUI wiring is validated by `quickstart.md` (Principle I exception).

**Target Platform**: Desktop — Linux, macOS, Windows.

**Project Type**: Desktop application (Rust + iced); render-free `lib` core + `gui` binary.

**Performance Goals**: No measurable idle cost. The cursor subscription is active **only while
the switcher is open**, so a closed switcher adds zero per-mouse-move work (SC-004).

**Constraints**: Must not fork a second forget path; must not alter feature 014's behavior; must
reuse the shared Material menu primitive.

**Scale/Scope**: A single menu with one item; tens of projects.

## Constitution Check

- [x] **I. Test-First (NON-NEGOTIABLE)**: All decision logic — the menu toggle/replace/anchor
  reducer arms and `clamp_menu_anchor` — is unit-tested in the render-free core
  (`tests/switcher_forget_menu.rs`, plus the event-mapping test in the binary). Only thin wiring
  (`mouse_area().on_right_press`, subscription registration, the clamped render) relies on the
  GUI-wiring exception, and it has a recorded `quickstart.md` procedure.
- [x] **II. Multi-Session Support**: No session state is touched. Forgetting still runs through
  feature 014's flow.
- [x] **III. Worktree Integration**: No worktree or filesystem operation is added. Forgetting
  remains a catalog-only action owned by feature 014.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: No new persisted state; nothing leaves the
  device. The new state (`cursor`, `window_size`, `project_menu_open`) is explicitly transient.
- [x] **V. Rust + iced Stack**: Rust + iced only. `ProjectMenu` bundles the path with its anchor
  so an "open menu with no position" is unrepresentable.
- [x] **VI. Cross-Platform Parity**: No OS-specific branching; positioning uses iced's own event
  and window APIs. CI covers all three platforms.
- [x] **VII. Documentation First-Class**: `docs/user-guide/project-selection.md` documents the
  right-click route in the same change, in both the switcher and Forgetting sections.
- [x] **VIII. Reusable UI Component Foundation**: Reuses the shared `MenuOverlay` (with its
  existing `.anchor(Point)` API) and `MenuItem`. `ProjectRow` gains one optional message field,
  matching its existing public-field data-carrier shape. `menu_panel_size` is added beside the
  component it describes so callers don't hardcode its dimensions. No widget is forked.

**Result**: PASS. No Complexity Tracking entries required.

## Project Structure

```text
src/
├── app.rs                      # ProjectMenu, clamp_menu_anchor; State.project_menu_open,
│                               #   .cursor, .window_size; CursorMoved / WindowResized /
│                               #   ProjectMenuToggled / ProjectMenuDismissed + reducer arms
│                               #   and popover mutual exclusion.
├── main.rs                     # cursor_move_events (only while the switcher is open),
│                               #   window resize_events, startup get_size task.
└── ui/
    ├── mod.rs                  # ProjectRow.on_context wiring; render the clamped,
    │                           #   cursor-anchored MenuOverlay emitting ProjectForgetRequested.
    └── material/
        ├── menu.rs             # menu_panel_size(items) for clamping.
        └── project_switcher.rs # ProjectRow.on_context + mouse_area right-press.

tests/
└── switcher_forget_menu.rs     # The pure surface (10 tests).

docs/user-guide/project-selection.md   # The right-click route.
```

**Structure Decision**: Purely additive on top of feature 014. No module boundary changes, no
new dependency, and no edit to feature 014's forget logic.

## Complexity Tracking

> No constitution violations — intentionally empty.
