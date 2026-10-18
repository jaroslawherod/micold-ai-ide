# Implementation Plan: Quick-Switcher to Jump to a Project, Location or Session

**Branch**: `claude/project-thread-ftdy5c` | **Date**: 2026-10-18 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/968-quick-switcher/spec.md`

## Summary

Ctrl/Cmd+Shift+P (physical P key, tier-1 reserved chord, 0 PTY bytes) toggles a quick-switcher: a
registered `Dialog`-layer surface holding the shared `Typeahead`, open, over the active project's
locations (#803 location order) each followed by its open sessions, then the other known projects.
Filtering is in-order character matching ranked prefix / run / scattered, decided in a new core
module `quick_switch` that reuses the type-ahead's folding and tier functions. Choosing resolves the
entry to the #803 `navigation::Step` and applies it through `apply_navigation` /
`finish_navigation`, so the result and the keyboard placement are exactly a row click's (or the
#803 chord's). Ledger D4 (active project only) is provisional: every "which entries exist" rule is
in one builder, `features/quick_switch.rs::candidates`, so option B is a change there.

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (`keyboard::key::Code::KeyP`); no new crate

**Storage**: none; the open switcher's state is per-window memory (Principle IV)

**Testing**: `cargo test` via `mise run test-core` (core `quick_switch`) and `mise run gate`
(client `tests/keymap.rs`, new `tests/quick_switch.rs`, `tests/overlay_registry.rs`,
`tests/navigation.rs`, docs test)

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: filtering 500 entries per keystroke under 100 ms in the test harness (SC-003)

**Constraints**: 0 PTY bytes for the chord (SC-002); no session started, stopped or closed (SC-005);
no daemon or wire change

**Scale/Scope**: 1 core module (+2 `pub(crate)` tier functions in `typeahead.rs`), 1 client feature
module, 1 view module, 1 keymap chord, 1 registry entry, 1 shared reader in `navigation.rs`, 2 doc
edits

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Red first: core `filter` (tiers, case, order, session-kept-by-location, non-ASCII), `reseat`, `step`, 500-entry timing; keymap decode/fallback/near misses/route/conflicts/docs labels; `candidates` from app state (filter, collapsed group, welcome screen, active project excluded, unavailable unselectable, closed session absent, current marks); U1 gate; U3 resolution per key incl. gone/unavailable; U4 restore; apply = #803's tested path; registry identity and cancellation. `ui/quick_switcher.rs` and `main.rs` focus/dispatch are glue covered by quickstart §B. |
| II. Multi-Session | PASS | The list is derived from live state on every view (FR-013); choosing only sends #803's `Selected` / `Reopened` / stop (SC-005 test); background sessions keep running. |
| III. Worktree Integration | PASS | Read-only over the sidebar's locations, Default included; missing/invalid worktrees are listed with their tag and chosen like a click; no worktree operation. |
| IV. Local-First | PASS | No persistence, no network. |
| V. Rust + iced | PASS | iced only; `Option<Open>` makes a closed switcher with a query unrepresentable; closed enums `EntryKey`, `Rank`, `Kind`; highlight held by key. |
| VI. Cross-Platform | PASS | `is_primary_shift` with the platform primary; physical-key match; CI on three platforms; quickstart §C. |
| VII. Documentation | PASS | `terminal-panes.md` row and `worktrees-and-sessions.md` bullet in the same PR; a test ties them to `switcher_chord_labels`. |
| VIII. Components | PASS | Composes the shared `Surface` (`SurfaceKind::Dialog`) with `material::dialog::fields`, `Typeahead` and `picker::Row` (its `details` line and `disabled`); no new widget, no fork. |

Post-design re-check: unchanged, all PASS.

## Design

### D1 — Rules (core) — R2, R3, R7, R9

New `crates/micold-core/src/quick_switch.rs` (in `lib.rs`): `EntryKey`, `Candidate`, `Rank`,
`Ranked`, `filter`, `reseat`, `step` ([data-model.md](./data-model.md)). `typeahead.rs`: `literal`
and `subsequence` become `pub(crate)` and take the folded name, so both modules share one
implementation of each tier.

### D2 — Keys — R1

`keymap.rs`: `PhysicalKey::KeyP`, `is_switcher_chord`, `switcher_chord_labels`,
`KeyOutput::ToggleSwitcher` in tier 1, `logical_physical` fallback for `p`/`P`.
`ui/material/terminal_pane.rs`: `to_keymap_physical` maps `Code::KeyP`; `KeyRouting::ToggleSwitcher`
publishes `Toggled { terminal_had_focus: true }` and captures. `app.rs`: `KeyRouting::ToggleSwitcher`,
`route_key`, `Message::QuickSwitch(quick_switch::Msg)`. `ui/mod.rs` `app_chord`: the chord →
`Toggled { terminal_had_focus: false }` ([contracts/keys.md](./contracts/keys.md)).

### D3 — Entries and state (client, render-free) — R4, R5, R7, R8

New `crates/micold-client/src/features/quick_switch.rs`: `Kind`, `Entry`, `candidates(&State)`,
`State`/`Open`, `Msg`, `update` with the U1 gate, U2 list, U3 resolution (returns
`Option<navigation::Step>`), U4 restore ([contracts/ui.md](./contracts/ui.md)).
`features/navigation.rs`: new `pub(crate) fn location_node(&State, &SessionLocation)` (name, tags,
`unfolded_sessions`, `main_session`), which `main_session_at` and `session_step` are refactored onto.
Root `State` gains `quick_switch: quick_switch::State`.

### D4 — Surface — R6

`features/quick_switch.rs`: `QuickSwitcherDialog` (`floating_surface!` on `Layer::Dialog`,
`Registered`). `overlay/registry.rs`: one `REGISTERED` line with `crate::ui::quick_switcher::dialog`.

### D5 — Applying a choice — R5

New `crates/micold-client/src/shell/quick_switch.rs`: routes `Message::QuickSwitch`. For `Chosen`
it calls `quick_switch::update`; when it returns a `Step` it runs the same sequence as
`shell/navigation.rs` (`apply_navigation`, dispatch the follow-ups through `update`,
`finish_navigation`) with the recorded `terminal_had_focus`; a `Stop` with the Overview open first
dispatches the Overview's close. That shared sequence is factored out of `shell/navigation.rs` into
one `apply_step(app, step, terminal_had_focus)` both call. On open it returns
`operation::focus(QUICK_SWITCHER_FIELD_ID)`. No branch of its own.

### D6 — View — R6

New `crates/micold-client/src/ui/quick_switcher.rs`: `dialog(&State) -> Element` (U6), and the
field id. Showcase: the dialogs section gains the quick-switcher with a query, an emphasised match,
a session row and an unavailable project.

### D7 — Docs — R10

`docs/user-guide/terminal-panes.md` §Keyboard shortcuts and
`docs/user-guide/worktrees-and-sessions.md` **Quick-switcher** bullet ([contracts/keys.md](./contracts/keys.md) K4).

## Requirement map

| FR / SC | Where | Research |
|---|---|---|
| FR-001, SC-002 | D2 tier 1, both routes; U1/U4 toggle | R1, R8 |
| FR-002 | D2 conflict test | R1 |
| FR-003, FR-004 | D3 `candidates` (U2) | R4 |
| FR-005 | D1 `filter`, `reseat` | R2, R3, R7 |
| FR-006 | D3 `Entry` context and `current`; D6 row | R6 |
| FR-007 | D1 `step`; D6 `on_move` / `on_pick` | R7 |
| FR-008, SC-004 | D3 U3 resolution; D5 #803 apply path | R5 |
| FR-009 | D4 Dialog dismissal (Escape, scrim); U4 | R6, R8 |
| FR-010 | D3 `terminal_had_focus`, `keyboard_row_before`; D5 `finish_navigation` | R5, R8 |
| FR-011, SC-005 | D5 only `Selected` / `Reopened` / stop; SC-005 count test | R5 |
| FR-012 | D3 U1 gate; #803 gate reads `registry::topmost` | R6, R8 |
| FR-013 | D3 derived list; D1 `reseat` by key; U3 "not listed" | R4, R7 |
| FR-014 | D6 `empty_message`; Enter intent needs an enabled highlighted row | R7 |
| FR-015 | all state on the window's `State` | — |
| FR-016 | D2 `PhysicalKey::KeyP`, fallback | R1 |
| FR-017, SC-006 | D7, docs test | R10 |
| SC-001 | D1 ranks (prefix first) + D3 order; quickstart B2–B3 | R2 |
| SC-003 | D1 timing test | R9 |
| Edge: Overview open | D5 `Selected` closes it; `Stop` closes it first | R5 |
| Edge: missing/invalid worktree | D3 lists with tag; choose = click | R4, R5 |
| Edge: welcome screen / no project | U2 table | R4 |

## Test strategy by layer

| Layer | Where | What it proves | FR |
|---|---|---|---|
| Core unit | `micold-core/src/quick_switch.rs` tests | empty query keeps order; `logpg` keeps Login page only; prefix > run > scattered, stable within; case ignored, `é` not folded to `e`; a matched location pulls in unmatched sessions after it with its rank, a matched session not duplicated; spans index the original name (non-ASCII); `reseat` keeps key, falls back to first selectable, skips unavailable, `None` when none; `step` skips unselectable and stops at ends; 500 candidates under 100 ms | 004, 005, 007, 013, SC-003 |
| Core unit | `typeahead.rs` existing tests | still green after the tier functions become shared | — |
| Client keys | `micold-client/tests/keymap.rs` | K5 | 001, 002, 016, 017, SC-002, SC-006 |
| Client state | `micold-client/tests/quick_switch.rs` (new) | `candidates`: filter-hidden worktree absent but current kept, collapsed group, closed session absent, other project's locations absent (D4), active project not a project entry, unavailable unselectable, welcome screen, names/tags/counts/current marks; U1 gate per surface and Settings; U3 per key incl. gone and unavailable (stays open); U4 restores `keyboard_row`; list change keeps query and highlight; apply through `apply_navigation` gives `session.active`, `keyboard_row`, reveal armed, both focus cases; session count unchanged (SC-005); two `State`s independent | 003–015, SC-004, SC-005 |
| Client state | `tests/navigation.rs` | still green after `location_node` refactor | — |
| Registry | `tests/overlay_registry.rs` | `QuickSwitcherDialog` identity, Dialog layer, cancellation = `Dismissed`; Escape and outside click reach it | 009 |
| Showcase | `tests/showcase_completeness.rs` | quick-switcher variant listed | VIII |
| Visual pass | quickstart §B (visual-pass skill) | B1–B12 | all |
| Platforms | CI on three; quickstart §C | parity, no system action, layouts | VI, 016 |

## Project Structure

### Documentation (this feature)

```text
specs/968-quick-switcher/
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
crates/micold-core/src/{quick_switch.rs (new), typeahead.rs, lib.rs}
crates/micold-client/src/
├── keymap.rs                     # PhysicalKey::KeyP, is_switcher_chord, ToggleSwitcher
├── app.rs                        # Message::QuickSwitch, KeyRouting::ToggleSwitcher, State field
├── main.rs                       # routes Message::QuickSwitch to shell/quick_switch.rs
├── features/{quick_switch.rs (new), navigation.rs, mod.rs}
├── overlay/registry.rs           # QuickSwitcherDialog registration
├── shell/{quick_switch.rs (new), navigation.rs, mod.rs}
├── ui/{quick_switcher.rs (new), mod.rs}
├── ui/material/terminal_pane.rs
└── showcase/{catalogue.rs, sections/*}
crates/micold-client/tests/{keymap.rs, quick_switch.rs (new), overlay_registry.rs, navigation.rs, showcase_completeness.rs}
docs/user-guide/{terminal-panes.md, worktrees-and-sessions.md}
```

**Structure Decision**: existing workspace layout; rules in `micold-core`, decoding in the client's
render-free `keymap.rs`, entries and state in `features/`, surface in the registry, apply in
`shell/`, rendering from the shared `ui/material` library in `ui/`.

## Complexity Tracking

No violations.
