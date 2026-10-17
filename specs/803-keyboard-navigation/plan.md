# Implementation Plan: Keyboard Navigation of Projects, Worktrees and Sessions

**Branch**: `claude/project-thread-ah7xxz` | **Date**: 2026-10-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/803-keyboard-navigation/spec.md`

## Summary

Six reserved chords (primary modifier + Shift + `[` `]` `,` `.` PageUp PageDown, matched by physical
key) step to the previous/next session of the current location, location of the project, and
available project, wrapping at both ends. The chords live in one table in `keymap.rs`
([contracts/keys.md](./contracts/keys.md) K1), decode in the terminal's tier 1 beside the Overview
chord (0 bytes to the PTY) and in the app's unfocused chord listener. A press resolves a `Step` from
the orders the sidebar and switcher already draw (pure `navigation::step` in core) and applies it
by sending the click's own message (`SessionMsg::Selected`, `ProjectMsg::Reopened`), plus a reveal
scroll and, when the terminal did not have the keyboard, a keyboard-focused sidebar row. A location
with no session becomes a navigation stop without changing the session shown. D4 (PageUp/PageDown)
and D5 (no direct jumps) are provisional; a different D4 answer changes K1 and the docs rows only.

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (`keyboard::Event::KeyPressed { physical_key, .. }`,
`key::Code`); no new crate

**Storage**: none; `nav_stop` and `keyboard_row` are per-window memory (Principle IV)

**Testing**: `cargo test` via `mise run test-core` (core `navigation`) and `mise run gate` (client
`tests/keymap.rs`, new `tests/navigation.rs`, sidebar state tests, `TreeItem` anatomy, showcase)

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: a press is one pass over the sidebar entries (tens of rows); no I/O

**Constraints**: 0 PTY bytes per chord (SC-002); no session started, stopped or closed (SC-005);
no daemon or wire change

**Scale/Scope**: 1 core module, 1 client feature module, keymap table + physical key, 2 sidebar
state fields, 1 shared component extension (`TreeItem::keyboard_focus`, `TakesTheKeyboard::shown`),
2 doc edits

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Red first: `navigation::step` table (core); `keymap` decode, fallback, near misses, route, conflicts, docs labels; `navigation::update` gate and step resolution from app state; apply (`Message::Navigate` → selected/stop/project, focus rule, reveal armed); sidebar `nav_stop`/`keyboard_row` clearing; `row_index`; `TreeItem::keyboard_focus` anatomy (layout-transparent, ring drawn). `main.rs` dispatch and the `ui/` row wiring are glue covered by quickstart §B. |
| II. Multi-Session | PASS | Orders are computed at each press from live state; a press never starts, stops or closes a session (SC-005 test); background sessions keep running; several helpers per location is a main test case. |
| III. Worktree Integration | PASS | Read-only over the sidebar's locations, Default included; missing/invalid worktrees are stops like others; no worktree operation. |
| IV. Local-First | PASS | No persistence, no network. |
| V. Rust + iced | PASS | iced only; closed enums `NavLevel`, `Dir`, `PhysicalKey`, `Step`, `RowRef`; `Option` makes "two rows hold the sidebar's keyboard" unrepresentable. |
| VI. Cross-Platform | PASS | One modifier rule (`is_primary_shift`) with the platform primary; physical-key match for layouts; CI on three platforms; quickstart §C. |
| VII. Documentation | PASS | `terminal-panes.md` table rows and `worktrees-and-sessions.md` bullet in the same PR; a test ties them to the table. |
| VIII. Components | PASS | Focus on a sidebar row extends the shared `TreeItem` builder (`keyboard_focus`) over the shared `TakesTheKeyboard` (`shown`); showcase gains a focused-row variant. |

Post-design re-check: unchanged, all PASS.

## Design

### D1 — Step rule (core) — R3

New `crates/micold-core/src/navigation.rs` (in `lib.rs`): `Dir`, `step` ([data-model.md](./data-model.md)).

### D2 — Keys — R1, R2, R9

`keymap.rs`: `PhysicalKey`, `KeyInput::physical`, `NavLevel`, `NavAction`, `NAV_CHORDS`,
`is_primary_shift` (factored from `is_primary_shift_chord`), `nav_chord`, `nav_chord_labels`,
`KeyOutput::Navigate`, tier-1 check in `encode`. `ui/material/terminal_pane.rs`:
`to_keymap_physical`, `KeyInput` built with it, `KeyRouting::Navigate` published and captured when
focused. `app.rs`: `KeyRouting::Navigate`, `route_key`, `Message::Navigate`. `ui/mod.rs`:
`overview_chord_listener`/`overview_chord` become `app_chord_listener`/`app_chord` answering both
([contracts/keys.md](./contracts/keys.md)). Every `KeyInput { .. }` literal gains `physical`.

### D3 — Resolving a press (client, render-free) — R3, R4, R6, R8

New `crates/micold-client/src/features/navigation.rs`: `Step`, `location_order(&State)`,
`update(&State, NavAction) -> Step` with the U1 gate and U2 table
([contracts/ui.md](./contracts/ui.md)).

### D4 — Sidebar state — R4, R7

`features/sidebar.rs`: `RowRef`, `State::nav_stop`, `State::keyboard_row`,
`SidebarMsg::KeyboardRowLeft`; `row_index(entries, &RowRef)` shared with `current_session_row`;
`reveal_scroll_offset` targets `keyboard_row` when set. `nav_stop` cleared in
`set_current_session`, `session::selected` and `project_entered`; `keyboard_row` cleared on
`KeyboardRowLeft` and on a pointer selection (U5).

### D5 — Applying a step — R5, R6, R7

`app.rs` `State::apply_navigation(step, terminal_had_focus) -> Vec<Message>`: pure, returns the
follow-up message (`Selected` / `Reopened`) and sets `nav_stop`, `keyboard_row`, released terminal,
reveal arm per U3. `main.rs` routes `Message::Navigate(action)`: reads `terminal_focused()`, calls
D3 then `apply_navigation`, and dispatches the follow-ups through its own `update` (so `Selected`
runs its full shell path). `keyboard_row` for a project step is set from the session the switch
restored, after `Reopened` ran.

### D6 — Component — R7

`ui/material/keyboard_focus.rs`: `TakesTheKeyboard::shown(bool)` (supplied focus draws the ring).
`ui/material/tree_view.rs`: `TreeItem::keyboard_focus(focused, on_change)` wraps the row in
`TakesTheKeyboard` (enabled only when set) with the row's indicator. `ui/sidebar.rs`: the row whose
`RowRef` is `keyboard_row` (location rows, session sub-rows, run rows) calls it with
`SidebarMsg::KeyboardRowLeft` on loss. Showcase: a tree with a keyboard-focused row.

### D7 — Docs — R10

`docs/user-guide/terminal-panes.md` §Keyboard shortcuts (three rows; intro covers both focus
states) and `docs/user-guide/worktrees-and-sessions.md` **Navigation** bullet ([contracts/keys.md](./contracts/keys.md) K4).

## Requirement map

| FR / SC | Where | Research |
|---|---|---|
| FR-001 | D2 `NAV_CHORDS`, D7 | R1 |
| FR-002 | D3 location row of U2, D5 `Selected` | R3, R5 |
| FR-003 | D3 session row of U2, D5 | R3, R5 |
| FR-004 | D3 project row of U2, D5 `Reopened` | R6 |
| FR-005 | D1 `step` | R3 |
| FR-006, SC-003 | D2 conflict test | R9 |
| FR-007, SC-002 | D2 tier 1, routing both paths | R2 |
| FR-008 | D2 `is_primary_shift`, near-miss tests | R2 |
| FR-009, SC-004 | D4 reveal target, D5 reveal arm, existing `effective_open` | R5, R7 |
| FR-010 | D5 focus rule, D4 `keyboard_row`, D6 | R7 |
| FR-011, SC-005 | D5 only sends `Selected`/`Reopened`; `Stop` starts nothing | R4 |
| FR-012 | D3 gate; `Selected` closes the Overview | R8 |
| FR-013, SC-006 | D7, docs test | R10 |
| FR-014 | nothing built (D5 provisional) | — |
| FR-015 | all state on the window's `State` | — |
| FR-016 | D2 `PhysicalKey`, `to_keymap_physical`, fallback | R1 |
| SC-001 | D1 wrap both directions | R3 |
| Edge: empty / no current / missing worktree | D1 rules, D3 `Stop`, D4 `nav_stop` | R3, R4 |
| Edge: welcome screen | D3 empty location order; project `current = None` | R6 |
| Edge: key repeat | each `KeyPressed` is a press | R2 |

## Test strategy by layer

| Layer | Where | What it proves | FR |
|---|---|---|---|
| Core unit | `micold-core/src/navigation.rs` tests | next/previous middle, wrap at both ends, one entry stays, empty → `None`, absent/`None` current → first/last | 005, SC-001 |
| Client keys | `micold-client/tests/keymap.rs` | K5: decode by physical key and by fallback (logical `{` `}` `<` `>`), both modifier sets, near misses incl. Ctrl+[ → `0x1b`, no overlap with reserved outputs, `route_key` both focus states, docs carry both labels of every chord | 001, 006–008, 013, 016, SC-002, SC-003, SC-006 |
| Client state | `micold-client/tests/navigation.rs` (new) | U1 gate (Settings, dialog, context menu, switcher panel); U2 for each level: filter skip, collapsed group skip, expanded group run included, helper order, unavailable project skipped, welcome screen, no current, one entry, `nav_stop` honoured and cleared; U3 apply: `Selected`/`Reopened` follow-up, reveal armed, focus rule both ways, `Stop` leaves `session.active` and the session count unchanged | 002–005, 009–012, 015, SC-004, SC-005 |
| Client state | `tests/features_sidebar.rs` / `sidebar_state.rs` | `row_index` for location/sub-row/run rows; `reveal_scroll_offset` to `keyboard_row`; `KeyboardRowLeft` clears; `current_session_writers.rs` still green | 009, 010 |
| Component | `ui/material/tree_view.rs`, `keyboard_focus.rs` tests | `keyboard_focus` adds no layout node size change; supplied+shown draws the ring; disabled rows untouched | 010 |
| Showcase | `tests/showcase_completeness.rs` | focused-row variant listed | VIII |
| Visual pass | quickstart §B (visual-pass skill) | B1–B12 | all |
| Platforms | CI on three; quickstart §C | parity, no system action | VI, 006 |

## Project Structure

### Documentation (this feature)

```text
specs/803-keyboard-navigation/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── keys.md
│   └── ui.md
└── tasks.md             # next: tasks unit
```

### Source Code (repository root)

```text
crates/micold-core/src/{navigation.rs (new), lib.rs}
crates/micold-client/src/
├── keymap.rs                     # PhysicalKey, NAV_CHORDS, nav_chord, Navigate
├── app.rs                        # Message::Navigate, KeyRouting::Navigate, apply_navigation
├── main.rs                       # routes Message::Navigate
├── features/{navigation.rs (new), sidebar.rs, session.rs, mod.rs}
├── ui/{mod.rs, sidebar.rs}
├── ui/material/{terminal_pane.rs, tree_view.rs, keyboard_focus.rs}
└── showcase/{catalogue.rs, sections/*}
crates/micold-client/tests/{keymap.rs, navigation.rs (new), features_sidebar.rs, showcase_completeness.rs}
docs/user-guide/{terminal-panes.md, worktrees-and-sessions.md}
```

**Structure Decision**: existing workspace layout; the wrap rule in `micold-core`, decoding in the
client's render-free `keymap.rs`, resolution and state in `features/`, rendering in the shared
`ui/material` library, wiring in `ui/` and `main.rs`.

## Complexity Tracking

No violations.
