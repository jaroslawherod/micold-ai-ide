# Implementation Plan: Attention Indicator on the Sidebar's Worktree Rows

**Branch**: `claude/project-thread-8dnq8h` | **Date**: 2026-10-05 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/575-workspace-attention-list/spec.md` (issue #575,
scope as changed by the user: an attention indicator on the sidebar's location rows).

## Summary

Each sidebar location row (every worktree row and the Default row) shows 039's shared unread mark
followed by the number of its counted sessions (unread, not closed, not in view), expanded or
collapsed. The count is derived on display from the sidebar's existing nodes through one new core
predicate, `micold_core::attention::counts_as_unread`, which the switcher's per-project count also
adopts, so closed sessions stop counting there too and the rows of a project add up to its
switcher count. Rendering reuses `UnreadMark` through a counted variant of `TreeItem`'s unread
state. No new state, no wire or daemon change, no persistence change.

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (workspace `Cargo.toml`); no new crate

**Storage**: none new. 039's persisted per-session `unread` flag is the only input (FR-006)

**Testing**: `cargo test` through `mise run test-core` (core) and `mise run gate` (client state
tests in `crates/micold-client/tests/`, in-crate geometry and contrast gates)

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: indicator correct within 1 s of a change (FR-007); recount is a filter over
one location's sessions per render

**Constraints**: row keeps its height; name shortens before the indicator (039 U8); no state
beyond 039's (FR-006); offline (Principle IV)

**Scale/Scope**: tens of sessions and worktrees per project; one core function, one sidebar
method, one `TreeItem` variant, two tooltip helpers, one showcase pose, one user-guide section

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every decision lives in tested code: `counts_as_unread` and `unread_session_count` in core unit tests; `SidebarEntry::unread_count`, `unread_tooltip_line`, `with_unread_line` in client state tests; `TreeItem::unread_count` rendering in `tree_view.rs` geometry tests. The `ui/sidebar.rs` wiring only calls those (glue exception). Tests are written red first per the TDD loop. |
| II. Multi-Session | PASS | Counts are over per-session state; nothing per session is added or shared; many sessions per location is the main case under test. |
| III. Worktree Integration | PASS | Read-only over existing locations; the Default row is a location row like a worktree row but keeps its own icon and is not styled as a worktree. No worktree operation. |
| IV. Local-First | PASS | Derived from local state; nothing stored or sent (FR-006). |
| V. Rust + iced | PASS | iced only; `RowUnread` enum makes "marked and counted" unrepresentable (R3). |
| VI. Cross-Platform | PASS | No platform code; CI runs the suite on all three; quickstart §C covers macOS, Windows and a sandboxed project. |
| VII. Documentation | PASS | `docs/user-guide/worktrees-and-sessions.md` § Unread sessions, and `docs/user-guide/project-selection.md` (switcher count skips closed sessions), in the same PR. |
| VIII. Components | PASS | Reuses `UnreadMark` and `TreeItem`; the new host option is a chainable builder method (`.unread_count(n)`); showcase updated. |

Post-design re-check: unchanged, all PASS.

**Known limitation (not a deviation)**: FR-005 asks the indicator to state its meaning to
assistive technology. iced 0.14 exposes no accessibility tree (no `accesskit` in `Cargo.lock`), so
no widget of this application can; the tooltip line carries the words (R5), as for 039's marks.

## Design

### D1 — The counted-session rule (core) — R1

`crates/micold-core/src/attention.rs`: add `pub fn counts_as_unread(session: &Session, in_view:
Option<SessionId>) -> bool`. `crates/micold-core/src/workspace.rs` `unread_session_count` filters
with it (adding `!archived`). `other_projects_unread` follows through `unread_session_count`.

### D2 — The location count (client, render-free) — R2

`crates/micold-client/src/features/sidebar.rs`: `impl SidebarEntry { pub fn unread_count(&self,
in_view: Option<SessionId>) -> usize }`, over `DefaultNode::sessions` / `WorktreeNode::sessions`
with `counts_as_unread`.

### D3 — The host (component) — R3, R4, R6

`crates/micold-client/src/ui/material/tree_view.rs`: private `enum RowUnread { Read, Unread,
Count(NonZeroUsize) }` replaces `pub unread: bool`; builder `.unread(bool)` unchanged; new builder
`.unread_count(usize)`. `Count(n)` pushes `UnreadMark::new(r).count(n.get()).role(label_role)`
where `Unread` pushes the bare mark, and does not select the emphasised label role. Every
caller already goes through `.unread(bool)` (`ui/sidebar.rs`, `showcase`, `anatomy_size.rs`,
`tree_view.rs` tests); only the render code in `tree_view.rs` reads the field. The field
becomes private (`pub(crate)` at most); the implementing task greps the workspace for direct reads
of `.unread` on a `TreeItem` and for `TreeItem { .. }` literals after the change.

### D4 — Tooltip words — R5

`crates/micold-client/src/features/sidebar.rs`: `unread_tooltip_line(n) -> Option<String>` and
`with_unread_line(tooltip: String, n) -> String`.

### D5 — Wiring (glue) — R2, R6

`crates/micold-client/src/ui/sidebar.rs` `build_items`: before matching the entry, take
`let unread = entry.unread_count(in_view)`; on the worktree row add `.unread_count(unread)` and
wrap the tooltip as `with_unread_line(worktree_tooltip(..), unread)`. `build_default_item` takes
the count as a parameter and does the same with `DEFAULT_LOCATION_LABEL`. Nothing else changes in
the row: tags, actions cluster, expansion.

### D6 — Showcase, contrast, docs

- `crates/micold-client/src/showcase/sections/atoms.rs::unread_mark`: three location-row poses
  (contract §Showcase). Catalogue entry text in `showcase/catalogue.rs` mentions them.
- `crates/micold-client/src/ui/material/composition_contrast.rs`: hovered-row fill for the mark,
  and a 4.5:1 text check for the count (R9).
- `docs/user-guide/worktrees-and-sessions.md` § Unread sessions: a paragraph on the indicator on
  worktree and Default rows — what it counts (not closed, not the one in view), that it shows
  collapsed and expanded, that viewing a session lowers it, hidden worktrees count only on the
  switcher (FR-014). `docs/user-guide/project-selection.md`: the switcher's unread count leaves out
  closed sessions (FR-010).

## Requirement map

| FR | Where | Research |
|---|---|---|
| FR-001 | D2, D3, D5; contract A1 | R2, R3 |
| FR-002 | D1, D2; contract A2; data-model "Counted session" | R1, R2 |
| FR-003 | D2 (count independent of `expanded`), D5; contract A3 | R2 |
| FR-004 | D3; contract A4, A5, A6 | R3, R4, R6 |
| FR-005 | D4, D5; contract A7; Known limitation | R5 |
| FR-006 | D1, D2 (derived, nothing stored); data-model | R2, R7 |
| FR-007 | D2 recomputed per render from catalog state | R7 |
| FR-008 | unchanged 039 path (one service, persisted `unread`) | R7 |
| FR-009 | no reducer for expand, collapse or hover touches attention state; test below | R7 |
| FR-010 | D1; contract A8 | R1, R8 |
| FR-011 | D3 keeps `.unread(bool)`; 039 tests stay green; contract A10 | R3 |
| FR-012 | D6 showcase | — |
| FR-013 | D6 contrast gate; contract A9 | R9 |
| FR-014 | D6 docs | — |
| FR-015 | no platform code; quickstart §C | R7 |

## Test strategy by layer

| Layer | Where | What it proves | FR |
|---|---|---|---|
| Core unit | `crates/micold-core/src/attention.rs` tests | `counts_as_unread`: unread counted; read, closed, in-view not | FR-002 |
| Core unit | `crates/micold-core/src/workspace.rs` tests | `unread_session_count` and `other_projects_unread` skip a closed unread session | FR-010 |
| Client state | `crates/micold-client/tests/sidebar_attention.rs` (new) | US1.1, US1.2, US1.4: `feature-x` 2, `feature-y` 0, Default 1; same count expanded and collapsed (FR-003); in-view session not counted; closed not counted (US1.5); tag-filtered worktree has no entry while the switcher counts it; feature 024 re-admitted row and a missing worktree carry counts; applying a catalog update that sets/clears `unread` changes the count (FR-007); sum of entries equals the active switcher count (FR-010); hover and expansion messages leave `unread` and the view report unchanged (FR-009); `unread_tooltip_line` / `with_unread_line` wording (FR-005) | FR-001–003, 005, 007, 009, 010 |
| Client state | `crates/micold-client/tests/switcher_unread.rs` | a closed unread session leaves the project count and the button total | FR-010 |
| Geometry gate | `crates/micold-client/src/ui/material/tree_view.rs` tests | counted row same height as plain row (one-line and with tags); at a narrow width the name is ellipsized and the indicator keeps its width; indicator before the trailing element and its bounds unchanged with the hover cluster shown; label role unchanged for `Count`; `.unread_count(0)` draws nothing | FR-004 |
| Contrast gate | `crates/micold-client/src/ui/material/composition_contrast.rs` | mark 3:1 and number 4.5:1 on rest, hover and selected fills, both schemes | FR-013 |
| Regression | existing 039 tests (`features_attention.rs`, `attention_*.rs`, `switcher_unread.rs`, `unread_mark.rs`, `anatomy_size.rs`) | session-row marks, switcher, button, notifications unchanged | FR-011 |
| Visual pass | quickstart §B (visual-pass skill), showcase B10 | look, hover, narrow width, both schemes, multi-window, restart | FR-004, FR-008, FR-012, SC-001–005 |
| Platforms | CI on Linux, macOS, Windows; quickstart §C | parity, sandboxed service | FR-015, SC-006 |
| Docs | `mise run gate` docs checks; review | user guide describes the indicator | FR-014 |

## Project Structure

### Documentation (this feature)

```text
specs/575-workspace-attention-list/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/attention-indicator.md
└── tasks.md             # next: tasks unit
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── attention.rs                 # + counts_as_unread
└── workspace.rs                 # unread_session_count uses it
crates/micold-client/src/
├── features/sidebar.rs          # + SidebarEntry::unread_count, unread_tooltip_line, with_unread_line
├── ui/material/tree_view.rs     # RowUnread, .unread_count(n)
├── ui/material/composition_contrast.rs
├── ui/sidebar.rs                # wiring in build_items / build_default_item
└── showcase/sections/atoms.rs   # location-row poses
crates/micold-client/tests/
├── sidebar_attention.rs         # new
└── switcher_unread.rs           # + closed-session cases
docs/user-guide/
├── worktrees-and-sessions.md    # § Unread sessions
└── project-selection.md         # switcher count skips closed sessions
```

**Structure Decision**: existing workspace layout; logic in `micold-core` and the client's
render-free `features/`, rendering in the shared `ui/material` library, wiring in `ui/sidebar.rs`.

## Complexity Tracking

None: no principle is violated.
