---
description: "Task list for feature 015 — forget a project from the switcher's right-click menu"
---

# Tasks: Forget a Project from the Switcher's Right-Click Menu

**Input**: Design documents from `/specs/015-switcher-forget-context-menu/`

**Tests**: MANDATORY per Constitution Principle I. The pure surface is unit-tested; the thin GUI
wiring uses the GUI-wiring exception and is validated by `quickstart.md`.

**Documentation**: Per Principle VII, the user-guide update ships in User Story 1.

## Phase 1: Foundational (shared by both stories)

- [X] T001 Add `ProjectMenu { path, anchor }` and the `State` fields `project_menu_open`, `cursor`, `window_size` in `src/app.rs`.
- [X] T002 Add the `CursorMoved`, `WindowResized`, `ProjectMenuToggled`, `ProjectMenuDismissed` messages in `src/app.rs`.
- [X] T003 Write failing tests for the menu reducer surface (open/close/replace, switcher stays open, popover mutual exclusion, hand-off to `ProjectForgetRequested`, dismissal forgets nothing) in `tests/switcher_forget_menu.rs`.
- [X] T004 Implement the reducer arms and add `project_menu_open = None` to `open_overlay` and to the `HelpMenuToggled` / `ProjectSwitcherToggled` / `SidebarFilterMenuToggled` / `WorktreeMenuToggled` arms in `src/app.rs`.

## Phase 2: User Story 1 — Forget from the switcher (P1) 🎯 MVP

- [X] T005 Add `on_context: Option<M>` to `ProjectRow` and wrap each rendered row in `mouse_area(..).on_right_press(..)` in `src/ui/material/project_switcher.rs`; the "Add project…" row carries none (FR-007).
- [X] T006 Wire `on_context: Some(Message::ProjectMenuToggled(path))` on switcher rows and render the one-item `MenuOverlay` emitting feature 014's `Message::ProjectForgetRequested(path)` in `src/ui/mod.rs` (FR-002/FR-003).
- [X] T007 Document the right-click route in `docs/user-guide/project-selection.md`, in both the switcher section and the Forgetting section (FR-011).
- [X] T008 Validate quickstart Scenarios A, B and D via `mise run run`.

## Phase 3: User Story 2 — Desktop-native placement (P2)

- [X] T009 Write failing tests for `clamp_menu_anchor` (inside / right edge / bottom edge / corner / flush / unknown window / window smaller than menu) and cursor anchoring + re-anchoring in `tests/switcher_forget_menu.rs`.
- [X] T010 Implement `clamp_menu_anchor` in `src/app.rs` and `menu_panel_size(items)` in `src/ui/material/menu.rs` (exported from `src/ui/material/mod.rs`).
- [X] T011 Anchor the menu at `ProjectMenu.anchor`, clamped at render time against `State.window_size`, in `src/ui/mod.rs` (FR-005/FR-006).
- [X] T012 Add `cursor_move_events` + `cursor_move_message` and subscribe **only while the switcher is open**; add `window::resize_events` and the startup `get_latest().and_then(get_size)` task in `src/main.rs` (FR-010).
- [X] T013 Add the binary-side test that cursor moves map to `CursorMoved` (with negative coordinates clamped) and other events are dropped, in `src/main.rs`.
- [X] T014 Validate quickstart Scenarios C and E via `mise run run`.

## Phase 4: Polish

- [X] T015 Verify `mise run test`, `cargo test --features gui --bin micold-ai-ide`, and `cargo clippy --features gui --all-targets -- -D warnings` are clean.
- [X] T016 Verify the build and full suite pass on macOS and Windows via CI (Principle VI). *(2026-08-20: satisfied by the three-OS CI matrix added in `10a1fe7` (2026-07-20) — `.github/workflows/ci.yml` builds the whole workspace and runs the render-free core suite plus the component gates on ubuntu/macos/windows for every code-affecting change, and has been green on all three since. Latest run: [32302430171](https://github.com/jaroslawherod/micold-ai-ide/actions/runs/32302430171). The full GUI suite and clippy stay Linux-only by design — that is the only runner with the iced system deps.)*

## Dependencies

- Phase 1 blocks both stories.
- US1 (T005–T008) is independently shippable: the menu works, positioned at a default anchor.
- US2 (T009–T014) refines placement; it depends only on Phase 1.
- T010 must precede T011 (the view calls both functions).

## Notes

- This feature adds **no** forget logic — feature 014 owns it. The menu item dispatches
  `ProjectForgetRequested` and nothing else.
- Validated on Linux; T016 remains open for macOS/Windows.
