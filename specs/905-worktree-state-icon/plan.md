# Implementation Plan: Worktree State Icon

**Branch**: `claude/project-thread-h057j5` | **Date**: 2026-10-17 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/905-worktree-state-icon/spec.md`

## Summary

Each worktree row and the Default row show one state icon — Running (triangle), Blocked (octagon)
or Idle / waiting (two bars), or an empty reserved slot when the state is unknown — derived from
the location's un-archived sessions. The only signal the window lacks today, whether an
`AwaitingInput` stop was mid-turn, already exists in the service's turn clock (feature 613) and is
carried out on `SessionSummary` (protocol 48). The rules are a pure core module; the icon is a new
shared component in a new `TreeItem` slot.

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (workspace `Cargo.toml`); no new crate; glyphs from the shipped
`assets/fonts/MaterialSymbolsOutlined.ttf`

**Storage**: none new; `paused_on_user` is runtime-only, like `activity` (Principle IV)

**Testing**: `cargo test` via `mise run test-core` (core) and `mise run gate` (daemon, client state
tests in `crates/micold-client/tests/`, in-crate geometry and contrast gates)

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: row state correct within 1 s of a service change (FR-006); derivation is a
`max` over one location's sessions per render

**Constraints**: row height and label x unchanged across states (FR-010); no new notification
path (FR-011); unknown never shown as a state (FR-005)

**Scale/Scope**: tens of sessions and worktrees per project; one core module, one turn-clock
reader, one wire field, one component, one `TreeItem` slot, three icons, one showcase entry, two
user-guide edits

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every decision is in tested code written red first: `session_state`/`location_state`/`tooltip_line` and `TurnClock::paused_on_user` (core unit); `note_activity`'s return and the overlay (daemon tests); `WorktreeNode::state`, `DefaultNode::state`, `with_state_line`, catalog adoption (client state tests); `TreeItem::state_icon` layout (geometry gate). `ui/sidebar.rs` wiring and the showcase render only call those (glue exception, quickstart §B). |
| II. Multi-Session | PASS | State is per session and combined per location by a pure `max`; no session's state is stored in or leaks into another; several sessions per location is a main test case. |
| III. Worktree Integration | PASS | Read-only over existing locations; the Default row gets the same icon but keeps its house icon and is not styled as a worktree (FR-002). No worktree operation. |
| IV. Local-First | PASS | Derived from the local service's state; nothing stored or sent off-device. |
| V. Rust + iced | PASS | iced only. `LocationState`/`BlockedReason` are closed enums with a derived `Ord` that encodes the precedence; `Icon` stays closed. The `paused_on_user` bool beside `activity` is justified in research R2. |
| VI. Cross-Platform | PASS | No platform code; glyphs from the one bundled font on all three; CI on all three; quickstart §C. |
| VII. Documentation | PASS | `docs/user-guide/worktrees-and-sessions.md` new § *Worktree state icon*; three rows in `docs/user-guide/icons.md`; same PR. |
| VIII. Components | PASS | New shared `StateIcon` with builder API (`new(state, roles).size(px)` → `.into()`); hosted by a `TreeItem` builder method (`.state_icon(..)`); showcase entry. Reuses `icon(..)`, `Icon`, `TreeItem`. |

Post-design re-check: unchanged, all PASS. Known limitation (not a deviation): no accessibility
tree in iced 0.14; the tooltip line carries the words (R9).

## Design

### D1 — Per-session and location state (core) — R4, R5

New `crates/micold-core/src/location_state.rs` (registered in `lib.rs`): `LocationState`,
`BlockedReason`, `session_state(&Session)`, `location_state(sessions)`,
`LocationState::tooltip_line()` — exactly the tables in [data-model.md](./data-model.md).

### D2 — Mid-turn flag (core + daemon + wire) — R1, R2, R3

- `crates/micold-core/src/attention.rs`: `TurnClock::paused_on_user()`.
- `crates/micold-core/src/protocol/messages.rs`: `SessionSummary::paused_on_user`
  (`serde(default)`); struct literals updated (`catalog.rs` `session_summary` sets `false`;
  test fixtures). `version.rs`: 47 → 48.
- `crates/micold-core/src/session.rs`: `Session::paused_on_user`, `false` in every constructor.
- `crates/micold-daemon/src/state.rs`: `overlay_live_summaries` sets it from `live.turn`;
  `note_activity` returns `changed || paused_changed` ([wire.md](./contracts/wire.md) W3).
- `crates/micold-client/src/catalog_sync.rs`: adopt it at both places `activity` is adopted.

### D3 — Icons — R6

`crates/micold-client/src/icons.rs`: `Icon::StateRunning` (`U+E037`), `StateBlocked` (`U+E160`),
`StateIdle` (`U+E034`), in `ALL`; pinned in `crates/micold-client/tests/icons.rs`.

### D4 — Component and host slot — R7

`crates/micold-client/src/ui/material/state_icon.rs` (`StateIcon`), and the `TreeItem` slot in
`tree_view.rs` ([state-icon.md](./contracts/state-icon.md) S1, S2).

### D5 — Render-free sidebar helpers — R4, R8

`crates/micold-client/src/features/sidebar.rs`: `WorktreeNode::state()`, `DefaultNode::state()`,
`with_state_line()` (S4).

### D6 — Wiring (glue)

`crates/micold-client/src/ui/sidebar.rs` `build_items` and `build_default_item` (S3).

### D7 — Showcase, contrast, docs

- `showcase/catalogue.rs` entry and `showcase/sections/atoms.rs::state_icon` (S6).
- `ui/material/composition_contrast.rs`: each glyph's role ≥ 3:1 on the sidebar row's rest,
  hovered and selected fills, both schemes (A6).
- `docs/user-guide/worktrees-and-sessions.md` § *Worktree state icon* (under *Browsing
  worktrees*): the three states, their icons, what puts a worktree in each (permission or question,
  failed, resumable; working, starting; idle, turn finished, no session), the empty slot for an
  unknown state, several sessions by precedence, the tooltip line. `docs/user-guide/icons.md`: three
  table rows (FR-013).

## Requirement map

| FR | Where | Research |
|---|---|---|
| FR-001 | D4, D6; S5 A1 | R7 |
| FR-002 | D6 (`build_default_item`), D5 `DefaultNode::state` | R7 |
| FR-003 | D1 `session_state`, `location_state` empty → Idle; D2 supplies the rule-5 flag | R1, R4 |
| FR-004 | D1 `location_state` (`Ord` max, archived skipped) | R4, R5 |
| FR-005 | D1 rule 7, D4 empty slot; S5 A2 | R4 |
| FR-006 | D2 `note_activity` broadcast; derived per render | R3 |
| FR-007 | D3 shapes, D7 contrast; S5 A5, A6 | R6 |
| FR-008 | D3 distinct codepoints; `tests/icons.rs` no duplicates | R6 |
| FR-009 | D1 `tooltip_line`, D5 `with_state_line`; S5 A7 | R8 |
| FR-010 | D4 fixed slot + `tag_indent`; S5 A3, A4 | R7 |
| FR-011 | no new message, notification or selection path; S5 A8; contract W5 | R2 |
| FR-012 | D4, D7 showcase | R7 |
| FR-013 | D7 docs | — |

## Test strategy by layer

| Layer | Where | What it proves | FR |
|---|---|---|---|
| Core unit | `crates/micold-core/src/location_state.rs` tests | every lifecycle × activity × `paused_on_user` gives the data-model row (SC-004, exhaustive over the closed sets); `location_state`: none → Idle, archived skipped, precedence pairs incl. Unknown beside Idle, blocked-reason order; `tooltip_line` words | 003, 004, 005, 009 |
| Core unit | `crates/micold-core/src/attention.rs` tests | `paused_on_user` true only after `Working → AskedUser`, false after `Working`, `Finished`, `PromptSubmitted`, and for `AskedUser` with no turn | 003 |
| Core unit | `crates/micold-core/tests/protocol_roundtrip.rs` | the field round-trips; a frame without it decodes `false` | 003 |
| Daemon | `crates/micold-daemon/src/state.rs` tests (beside the `note_activity` ones) | W3 table: returns `true` on `Notification`→`Stop`; overlay sets the flag from the live clock | 006 |
| Client state | `crates/micold-client/tests/sidebar_state_icon.rs` (new) | snapshot with `paused_on_user` reaches `Session` (both catalog paths); `WorktreeNode::state`/`DefaultNode::state` over mixed sessions, expanded and collapsed; filtered-out worktree has no entry; missing worktree still has a state; a snapshot change changes the state with no other message; `with_state_line` order before `with_unread_line` | 001, 002, 004, 006, 009, 011 |
| Geometry gate | `crates/micold-client/src/ui/material/tree_view.rs` tests | label x and row height equal for the four states, one-line and with tags; tag line starts under the label with the slot; narrow width ellipsizes the name, slot keeps width; row without the slot unchanged | 010, SC-005 |
| Contrast gate | `crates/micold-client/src/ui/material/composition_contrast.rs` | three glyph roles ≥ 3:1 on rest, hover, selected, both schemes | 007 |
| Icons | `crates/micold-client/tests/icons.rs`, `icons_font.rs` | pinned codepoints, no duplicates, present in the font | 007, 008 |
| Showcase | `crates/micold-client/tests/showcase_completeness.rs` | `StateIcon` entry lists its variants | 012 |
| Visual pass | quickstart §B (visual-pass skill) | look, greyscale shapes, both schemes, live change, restart, narrow, two windows | 001, 002, 006, 007, SC-001–003 |
| Platforms | CI on Linux, macOS, Windows; quickstart §C | parity | Principle VI |
| Docs | gate docs checks; review | user guide describes states and icons | 013 |

## Project Structure

### Documentation (this feature)

```text
specs/905-worktree-state-icon/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── state-icon.md
│   └── wire.md
└── tasks.md             # next: tasks unit
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── location_state.rs            # new: LocationState, BlockedReason, session_state, location_state
├── lib.rs                       # + pub mod location_state
├── attention.rs                 # + TurnClock::paused_on_user
├── session.rs                   # + Session::paused_on_user
└── protocol/{messages,version}.rs  # + SessionSummary::paused_on_user; protocol 48
crates/micold-core/tests/protocol_roundtrip.rs
crates/micold-daemon/src/
├── state.rs                     # overlay + note_activity return
└── catalog.rs                   # session_summary: false
crates/micold-client/src/
├── icons.rs                     # + StateRunning, StateBlocked, StateIdle
├── catalog_sync.rs              # adopt paused_on_user
├── features/sidebar.rs          # + state(), with_state_line
├── ui/material/state_icon.rs    # new: StateIcon
├── ui/material/{mod,tree_view,composition_contrast}.rs
├── ui/sidebar.rs                # wiring
└── showcase/{catalogue.rs,sections/atoms.rs}
crates/micold-client/tests/{sidebar_state_icon.rs (new),icons.rs,showcase_completeness.rs}
docs/user-guide/{worktrees-and-sessions.md,icons.md}
```

**Structure Decision**: existing workspace layout; rules in `micold-core`, the signal in the
daemon, render-free helpers in the client's `features/`, rendering in the shared `ui/material`
library, wiring in `ui/sidebar.rs`.

## Complexity Tracking

No violations.
