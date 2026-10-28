# Implementation Plan: Back/Forward Navigation (Mouse Side Buttons and Alt+←/→)

**Branch**: `claude/project-thread-e4cve5` | **Date**: 2026-10-27 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/1465-back-forward-navigation/spec.md`

## Summary

One navigation history per window, recorded by **observing** the shown place after every message
(the binary's post-update hook, where `follow_header` and `report_window_view` already run), so
every way of moving, the app's own included, is covered by one call (research R1). The list rules
(record, truncate, cap 50, step skipping and removing gone entries, never landing on the shown
place) are a generic `History<P>` in a new core module `history` (R4). A press — the mouse's
Back/Forward side buttons from a window-wide `listen_with` that ignores capture (R7), or Alt+←/→
(Cmd+[ / Cmd+] on macOS) from the app-chord listener, refused while a terminal or a live field
holds the keyboard (R8) — resolves in a render-free client feature `history` to the Review tab's own
**Back** (R9), a target place, or nothing. Target existence is decided before anything moves, with
an injected worktree lookup for other projects (R5); showing it sends the messages the existing
controls send, through the root `update`, with observation suspended until it lands (R2, R6).

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (`mouse::Button::{Back, Forward}`, `event::listen_with`,
`keyboard::listen`); winit 0.30 maps the side buttons on all three platforms (R7); no new crate

**Storage**: none; the history is per-window memory (FR-001, Principle IV)

**Testing**: `cargo test` via `mise run test-core` (core `history`) and `mise run gate` (client
`tests/keymap.rs`, new `tests/history.rs`, `tests/review_keys.rs`, `ui/app_chord_tests.rs`,
`shell/subscriptions.rs` unit tests, `terminal_pane.rs` widget tests, `main_tests.rs`, docs test)

**Target Platform**: Linux (X11, Wayland), macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: observing costs one place derivation and one comparison per message; a press
walks at most 50 entries, with one local `git worktree list` per other project consulted

**Constraints**: 0 PTY bytes for a side-button press (FR-008); a focused field or terminal keeps the
keys byte-for-byte (SC-003); no daemon or wire change; no persistence

**Scale/Scope**: 1 core module, 1 client feature module, 1 shell module, 1 subscription function,
3 keymap functions, 5 new root methods, 2 extracted shared helpers (`navigation::blocked`,
`State::live_focused_field`), 2 sheet rows, 1 docs table edit

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Red first: core `History` (record, truncation, cap, step, removal, merge, SC-002 sweep); keymap H1/H2/near misses/conflicts/docs labels; `app_chord` and `overview_arrow`; `side_button` with both statuses; terminal pane side-button presses; `tests/history.rs` (observe per mover, gate rows, Review back, gone targets with an injected lookup, `apply_history` messages, `moving`); `main_tests.rs` end to end through the real root `update`; sheet rows. Glue without decisions (`shell/history.rs` dispatch, the subscription push, the `main.rs` hook call) is covered by those and quickstart §B. |
| II. Multi-Session | PASS | The history holds references (ids, locations), never session state; background sessions starting or ending record nothing unless the shown place changes (U1); a gone session is skipped, never resurrected. |
| III. Worktree Integration | PASS | Read-only: existence reads the active project's listed worktrees or a local discovery of another project's (the same read a switch runs); Default is always a location; no worktree operation. |
| IV. Local-First | PASS | No persistence, no network. |
| V. Rust + iced | PASS | iced only. `Place`/`View` make a place without a project, a Settings "place" or a session without its location unrepresentable; `History`'s fields are private so the cap and no-equal-neighbours invariants hold by construction. |
| VI. Cross-Platform | PASS | Side buttons through winit's uniform `Back`/`Forward` (X11, Wayland, Windows, macOS); the one key split is `cfg(target_os = "macos")` inside `keymap.rs` beside `is_primary_shift`; CI on three platforms; quickstart §C. |
| VII. Documentation | PASS | `docs/user-guide/terminal-panes.md` app-chord table and intro in the same PR; a test ties them to `history_key_labels` (FR-013). |
| VIII. Components | PASS | No new widget: the Review sheet gains two rows through its existing `SheetModel`; no other UI. |

Post-design re-check: unchanged, all PASS.

## Design

### D1 — History rules (core) — R4

New `crates/micold-core/src/history.rs` (in `lib.rs`): `Dir`, `View`, `Place`, `History<P>` with
`CAP = 50`, `record`, `step`, `current`, `entries`, `position` ([data-model.md](./data-model.md)).

### D2 — Keys — R8, R11

`keymap.rs`: `history_key`, `history_key_labels`, `history_key_label`
([contracts/keys.md](./contracts/keys.md) H1). `ui/mod.rs`: `app_chord` maps it to
`Message::History(Pressed { dir, source: Key })`; `overview_arrow` skips it (H3).

### D3 — Side buttons — R7

`shell/subscriptions.rs`: `side_button` and an always-on `iced::event::listen_with(side_button)` in
the subscription batch (H4).

### D4 — Feature state and resolution (client, render-free) — R5, R8, R9, R10

New `crates/micold-client/src/features/history.rs`: `State { list, moving }`, `Source`, `Msg`,
`Step`, `resolve`, `exists`, `observe`, `finish` ([contracts/ui.md](./contracts/ui.md) U3, U5).
Root `State` gains `history: history::State`; `Message::History(history::Msg)`.
Extracted for reuse, not copied: `features::navigation::blocked(&State)` (the first line of
`navigation::update`, which then calls it) and the root's `State::live_focused_field()`
(the field rule inside `changes::keys::review_holder`, which then calls it). The meta-tests that
register a feature are updated with it: `tests/feature_registration_cost.rs` (the feature list and
`ALLOWED_CROSS_FEATURE_NAMES` entries for `history` → `navigation` (`blocked`), `changes`
(`can_go_back`)), `tests/feature_write_isolation.rs` and
`tests/features_are_render_free.rs` as they require, and the per-feature isolation test
`tests/features_history.rs` (`every_feature_module_has_an_isolation_test`).

### D5 — Root: derive, observe, apply — R1, R2, R3, R6

`app.rs`: `State::shown_place`, `press_history`, `observe_place`, `finish_history_move`,
`apply_history` (U1–U4); `app.rs` is the only caller of the feature's functions.
`main.rs` `update`: `app.core.observe_place()` after `update_inner`, beside `follow_header`; routes
`Message::History` to the new `shell/history.rs` (shape B: `pub fn update(&mut App,
history::Msg) -> Task<Message>`), which calls `app.core.press_history` with
`lookup = |p| discover_worktrees(git, p)` mapped to paths, dispatches `apply_history`'s messages
through `crate::update`, then `finish_history_move` (U2); it refreshes availability first (U2 step
0, R5). `features/runs.rs`: `State.known`, filled in `routed` on `GroupsChanged` and read on
`CompareOpened` for a project just switched to (R5). `State::update` declines
`Message::History` (shape B, as `Navigation`).

### D6 — Review sheet — R11

`features/changes/keys.rs::sheet_model` appends the two U6 rows from `keymap::history_key_label`.
The showcase's two sheet poses (`showcase/sections/review.rs`) pick them up unchanged.

### D7 — Docs — R11

`docs/user-guide/terminal-panes.md` §Keyboard shortcuts: rows "Back (or the mouse back button)"
`Alt+←` / `Cmd+[` and "Forward (or the mouse forward button)" `Alt+→` / `Cmd+]`; the intro's "The
app takes every chord below" gains the exception: a focused text field or terminal keeps Back and
Forward's keys, and the side buttons work everywhere. No other guide page changes (spec *Out of
Scope*).

## Requirement map

| FR / SC | Where | Research |
|---|---|---|
| FR-001 | D4 `State.list`, root field | R4 |
| FR-002 | D5 `shown_place` + `observe_place` after every message | R1, R3 |
| FR-003, FR-004, FR-005, SC-004 | D1 `record` | R4 |
| FR-006, SC-002 | D1 `step`; D4 `exists`; D5 `apply_history` | R5, R6 |
| FR-007 | D1 `step` removes; D4 `moving`; D5 `finish_history_move` | R2 |
| FR-008 | D3 status ignored; terminal pane reports L/M/R only | R7 |
| FR-009, SC-003 | D2 not in `encode`; D4 key gate (`terminal_focused`, `live_focused_field`) | R8 |
| FR-010 | D4 `navigation::blocked` | R10 |
| FR-011, SC-005 | D4 `ReviewBack`; D5 `BrowseBack` | R9 |
| FR-012 | D2 each `KeyPressed`; D3 each `ButtonPressed` | R7, R8 |
| FR-013 | D7, docs test | R11 |
| FR-014 | D6 | R11 |
| FR-015 | test strategy below | R4 |
| SC-001 | `main_tests.rs`; quickstart B1, C1–C3 | R7, R8 |
| Edge: cross-project | D5 `Reopened` first, availability refreshed before `exists`; R2 suppression; `runs.known` for a Compare target | R2, R5 |
| Edge: closed Review tab | U4 `ReviewChangesRequested` reopens it | R6 |
| Edge: app moves | D5 observe after every message | R1 |

## Test strategy by layer

| Layer | Where | What it proves | FR |
|---|---|---|---|
| Core unit | `micold-core/src/history.rs` tests | U7 core list: record/equal/truncate/cap, step ends, removal, neighbour merge, never `shown`, SC-002 sweep over every gone-subset of five entries | 003–007, SC-002, SC-004 |
| Client keys | `tests/keymap.rs` | H5: decode per platform, near misses, `encode` unchanged (SC-003), no chord conflicts, docs labels | 009, 012, 013, SC-003 |
| Client listeners | `ui/app_chord_tests.rs`, `shell/subscriptions.rs` tests | H3 `app_chord`, `overview_arrow` skip; H4 `side_button` with Captured and Ignored | 008, 009 |
| Terminal | `ui/material/terminal_pane.rs` widget tests | Back/Forward presses publish nothing reaching the process | 008 |
| Client state | `tests/history.rs` (new) | U7 client list: observe per mover, gate rows, Review back, gone targets (injected lookup), apply messages, `moving`, two `State`s independent | 001–011 |
| Client state | `tests/review_keys.rs`, `tests/features_navigation.rs`, existing review-holder tests | sheet rows on/off; still green after the `blocked` and `live_focused_field` extractions | 014 |
| Binary | `main_tests.rs` | end to end through `update`: US1 AS3, cross-project Back without an intermediate entry, Overview/Compare/Review targets, cross-project Compare | 002, 004, 006, 007, SC-001 |
| Meta | `tests/features_history.rs` (new), `feature_registration_cost.rs`, `feature_write_isolation.rs`, `features_are_render_free.rs` | the feature's types construct alone; it is registered, root-driven and render-free | — |
| Visual pass | quickstart §B (visual-pass skill) | B1–B13 | all |
| Platforms | CI on three; quickstart §C | side buttons and keys per platform | VI |

## Project Structure

### Documentation (this feature)

```text
specs/1465-back-forward-navigation/
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
crates/micold-core/src/{history.rs (new), lib.rs}
crates/micold-client/src/
├── keymap.rs                     # history_key, history_key_labels, history_key_label
├── app.rs                        # Message::History, history field, shown_place, live_focused_field, observe_place,
│                                 # finish_history_move, apply_history
├── main.rs                       # observe_place hook; routes Message::History
├── main_tests.rs                 # end-to-end presses
├── features/{history.rs (new), mod.rs, navigation.rs (blocked),
│             changes/keys.rs (review_holder onto live_focused_field; sheet_model rows)}
├── shell/{history.rs (new), mod.rs, subscriptions.rs (side_button)}
├── ui/{mod.rs (app_chord, overview_arrow), app_chord_tests.rs}
└── ui/material/terminal_pane.rs  # side-button tests only
crates/micold-client/tests/{history.rs (new), features_history.rs (new), keymap.rs, review_keys.rs, feature_registration_cost.rs, …}
docs/user-guide/terminal-panes.md
```

**Structure Decision**: existing workspace layout; list rules in `micold-core`, key decoding in the
client's render-free `keymap.rs`, resolution and state in `features/`, derivation and apply at the
root (`app.rs`, which may name every feature), dispatch in `shell/`, the listeners where the other
listeners are.

## Complexity Tracking

No violations.
