# Implementation Plan: Worktree context header and session tabs above the terminal

**Branch**: `claude/project-thread-y28cqn` | **Date**: 2026-10-24 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1126-session-context-header/spec.md`

## Summary

The session area gains a **context header** (title, state chip, branch chip, pull request chip,
Changes button) and, under it, a **tab strip** (one session tab per session of the location, the
shown session's shell tabs and "+" after its tab, status and restart, **New session**, the split
buttons); the bar under the terminal is removed. The header's words are pure rules in a new core
module `micold_core::context_header`, fed by the Overview card's own `CardModel` for the location
(so title and state equal the card's), the sidebar's pull request reading, and a new background read
of the Changes view's list for the shown location (`features::changes::HeaderCounts`). The strip's
members, order, marks and arithmetic are a new render-free module `features::session_strip`, into
which the bar's predicates move from `ui/terminal.rs`. The view is a new `ui/session_area.rs`;
`ui/mod.rs` changes only in the branches that draw an active session and the Changes view (its tab
menu reaches the moved `StripTab` through a re-export in `ui/terminal.rs`). Design decisions:
[research.md](research.md).

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (`row`, `column`, `container`, `advanced::Widget` for the
promoted keyed row); notify (existing watch); no new crate

**Storage**: none. Counts and the strip model are derived in memory; nothing persisted or sent
(Principle IV)

**Testing**: `micold-core` unit tests (`mise run test-core`) for the header text; client lib tests for
`features::session_strip`, `features::changes` (header counts), `cdk::keyed`, `session_area`
press choice; client integration tests (`features_changes.rs`, `session_area_stability.rs`,
`terminal_tabs.rs`, `tab_reveal.rs`, `terminal_focus.rs`, `keyboard.rs`); geometry gates in
`tests/layout_snapshot.rs` and a regenerated fixture; visual pass (quickstart §B). Whole gate:
`scripts/gate.sh`

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: header and strip derived per render, O(sessions + shell instances); one
`git diff` list read per debounced burst of edits in the shown location (R6)

**Constraints**: no overlap at 640×480 with the 300 px sidebar (session area 340 px) or at 1280×800;
press acts on the first try while members change; pane chords unchanged; no new GitHub read

**Scale/Scope**: core: 1 new module, 1 new constructor (`StatusPhrase::of_status`); client: 2 new
modules (`features/session_strip.rs`, `ui/session_area.rs`), changed `ui/terminal.rs` (bar removed,
predicates moved), `ui/mod.rs` (one branch), `ui/overview.rs` (`location_card`),
`features/changes.rs` (header counts), `app.rs` (follow step), `shell/{changes,issues,subscriptions}.rs`,
`ui/material/tag.rs` (`leading`, `max_width`), `ui/material/tab.rs` (`SESSION_WIDTH`),
`ui/cdk/keyed.rs` (promoted), `keymap.rs` (one label); tests and gates migrated; docs: two user-guide
pages and the layout-snapshot page

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every rule is red-first: header text in `micold-core` (contract H1–H6); strip members, marks, stops, presses and arithmetic in render-free `features::session_strip` (S1–S6); header counts transitions in `features::changes` (data-model); follow step through `State::update` (H7); keyed slots in `cdk::keyed`; Changes press choice as a pure function (H8). The header and strip widgets only lay these out (glue), checked by geometry gates and quickstart §B. |
| II. Multi-Session | PASS | One session tab per session; a member's stopped mark is asked of its own process (S3); another session's shell tabs are never drawn (S1); the header names the location, so switching sessions there changes only the marked tab. No per-session state added; nothing persisted. |
| III. Worktree Integration | PASS | The counts read the shown location's own directory with the view's reader and scope rule (Default: uncommitted only; worktree: against its review base), R5. No worktree is created, changed or removed; **New session** runs the existing confirmed start. |
| IV. Local-First | PASS | No new network call: the pull request chip reads the existing status reading, the title the existing issue lookup on its existing switch (R3, R4); counts are a local git read, refused (no counts) where the service runs on another computer. |
| V. Rust + iced | PASS | iced only. `Member` and `StripTab` are closed enums, so "exactly one marked" is total (S2); `CountsReading::Unknown` makes a stale figure unrepresentable in the label (R5). |
| VI. Cross-Platform | PASS | No platform code; the "+" tooltip's chord comes from `keymap` per platform (R13); fonts are bundled, so gates measure what each platform draws; CI on three platforms. |
| VII. Documentation | PASS | `docs/user-guide/terminal-panes.md` and `docs/user-guide/reviewing-changes.md` updated, `docs/development/layout-snapshot.md` *What is covered* lists the new states, in the same PR (FR-015). |
| VIII. Components | PASS | `Tag` extended (`.leading`, `.max_width`), `Tab` gains `SESSION_WIDTH`, `Button::selected` reused for Changes, `KeyedColumn` promoted to `cdk::Keyed` with an axis rather than a second keyed widget; all builders end in `.into()`; colours from `Roles` and the card's `palette`. |

Post-design re-check: unchanged, all PASS. No Complexity Tracking entries.

## Design

### Core (`crates/micold-core`)

- **`src/context_header.rs`** (new, `pub mod context_header` in `lib.rs`): `ChangeSummary`,
  `CountsReading`, `PrChipInput`, `Chip`, `HeaderText::of`. Contract:
  [contracts/context-header.md](contracts/context-header.md) H1–H6.
- **`src/pr_list.rs`**: `StatusPhrase::of_status(&PullRequestStatus)`, sharing `of`'s candidate list
  (R4).

### Client (`crates/micold-client`)

- **`src/features/session_strip.rs`** (new): `StripModel::of`, `Member`, `MemberKey`,
  `member_offsets`, and, moved from `ui/terminal.rs` with their tests, `StripTab`, `marked_tab`,
  `process_stopped`, `session_status`, `restart_message`, `attached_process_restartable`,
  `overflowing`, `strip_overflow`, `scroll_into_view`, `marked_tab_index` (R7, R8). Contract:
  [contracts/session-area.md](contracts/session-area.md) S1–S6.
- **`src/features/changes.rs`**: `State.header: HeaderCounts`, `Msg::HeaderFollowed`,
  `Msg::HeaderCountsRead`, `Effect::ReadCounts`; `Msg::Changed` and `Msg::WorktreesListed` also act
  on the header (data-model *HeaderCounts*, R5).
- **`src/app.rs`**: the follow step at the end of `State::update` (R6).
- **`src/shell/changes.rs`**: runs `ReadCounts` with `local` + `read_list` under
  `Toggles::default()`, `ChangeSummary::of` off the update thread (R5).
- **`src/shell/subscriptions.rs`**: also watches the header's entry while a session is shown and the
  Overview is closed (R6).
- **`src/shell/issues.rs`**: `request_titles(app, dirs)` from `on_overview_opened`; called for the
  shown worktree when the header follows a new location (R3).
- **`src/ui/overview.rs`**: `pub(crate) fn location_card(state, &SessionLocation) -> Option<CardModel>`
  (R2).
- **`src/ui/session_area.rs`** (new): `view` (header, strip, body order, A1), `header` (H9),
  `strip` (S7), `changes_press` (H8).
- **`src/ui/terminal.rs`**: `pane` returns the pane area only (the bottom bar, `tab_strip_row`,
  `pinned_ai_tab`, `right_aligned_tabs`, `leading_slack`, `natural_strip_width` removed); imports and
  re-exports (`pub use`) the moved predicates and `StripTab`, so `ui/mod.rs`'s tab menu and
  `tests/{app_state,popover_displacement,overlay_registration}.rs` keep their paths (R7).
- **`src/features/session.rs`**: `shell_instance_menu`, `strip_tab_menu_requested` and
  `Msg::StripTabMenuRequested` take `features::session_strip::StripTab` (R7).
- **`src/ui/mod.rs`**: the active-session branch of `view_with` calls `session_area::view`; the
  Changes branch draws under the header when a session is shown (R12).
- **`src/ui/material/tag.rs`**: `.leading(Element)`, `.max_width(f32)` (R11).
- **`src/ui/material/tab.rs`**: `SESSION_LABEL_WIDTH`, `SESSION_WIDTH` (derived, R8).
- **`src/ui/cdk/keyed.rs`** (promoted from `ui/transcript/keyed.rs`): `Keyed::row` / `Keyed::column`;
  the transcript imports it (R10).
- **`src/keymap.rs`**: `new_terminal_chord_label()` (R13).
- **`src/showcase/`**: the strip's session tab and the header's chips shown in the catalogue
  (`showcase_completeness`).

### Tests and gates (`crates/micold-client/tests`)

- `terminal_bar_stability.rs` → `session_area_stability.rs` (S8); `terminal_tabs.rs`, `tab_reveal.rs`
  retargeted to the strip; `features_changes.rs` gains H7.
- `gates/bar_controls_hold_their_size.rs` → `gates/strip_controls_hold_their_size.rs`;
  `gates/tabs_anchor_the_trailing_edge.rs` retired for `gates/strip_controls_anchor_the_trailing_edge.rs`;
  `gates/tab_children_fit.rs` re-anchored; `gates/containment.rs`'s strip-overflow case retargeted
  to `session-area-twelve-tabs` and the strip's path (R14); new `gates/session_area_fits.rs` (S9,
  H10, A1).
- `support/covered_states.rs`: `session-area-one-tab`, `session-area-six-tabs`,
  `session-area-twelve-tabs`, `session-area-changes-open`; `fixtures/layout_snapshot.txt` regenerated
  (R14).

### Docs

`docs/user-guide/terminal-panes.md` (*Split and pick a terminal*: the strip above the terminal, New
session, status and restart; no bar under the terminal), `docs/user-guide/reviewing-changes.md`
(*Opening the Changes view*: the header's Changes button, its counts, pressing it again to close),
`docs/development/layout-snapshot.md` (*What is covered*).

## Requirement map

| FR | Where | Test layer |
|---|---|---|
| FR-001 header, strip, panes; nothing under | `session_area::view`, A1, R12 | gate `session_area_fits` (nothing below the pane area); covered states; §B B1 |
| FR-002 title | `HeaderText.title` from `location_card`, H1, R2 | core unit; client: `location_card` equals the Overview's card for the same location |
| FR-003 state chip, more than colour | `HeaderText.state`, card `palette` + `state_icon` + word, H2, R11 | core unit; §B B1, B12 |
| FR-004 branch chip | `HeaderText.branch`, H3 | core unit |
| FR-005 pull request chip | `PrChipInput` from `pr_status`, `StatusPhrase::of_status`, H4, R4 | core unit (incl. `of`/`of_status` agreement); client unit: no input while status is off |
| FR-006 Changes counts, none, unknown | `ChangeSummary`, `CountsReading`, H5, H6, R5 | core unit; client: `HeaderCounts` transitions (stale seq, failure, re-read hides the last figure) |
| FR-007 open, shown open, close | `changes_press`, `Button::selected`, H8 | client unit; §B B6 |
| FR-008 strip members, New session, splits | `StripModel::of`, S1, S5, S7 | client unit; gate `strip_controls_anchor_the_trailing_edge`; §B B1, B3 |
| FR-009 nothing the bar offered is lost | S3–S5 (incl. the tab menu), R7, R9 | client unit (status, restart target, presses, menu *Restart* iff stopped per member); `tab_reveal.rs`; `popover_displacement.rs`, `overlay_registration.rs`; §B B2–B5 |
| FR-010 scroll, fixed controls, edge cue | `member_offsets`, `strip_overflow`, `scroll_into_view`, S6, R8, R9 | client unit; gates `strip_controls_hold_their_size`, `tab_children_fit`; covered state twelve tabs; §B B10 |
| FR-011 press on first try, on that control | fixed child lists, keyed row, S8, R10 | `session_area_stability.rs`; `cdk::keyed` slot test |
| FR-012 chords and focus unchanged | same messages as the bar (S5); `KeyboardElsewhere` unchanged | `terminal_focus.rs`, `keyboard.rs`, `navigation.rs`, lib `app_chord_tests` kept green; §B B11 |
| FR-013 ellipsis + tooltip; no overlap at minimum width | `Ellipsized`, `Tag::max_width`, tooltips, H10, S9 | gate `session_area_fits` at 640×480 and 1280×800, light and dark, 1/6/12 members; `layout_text_overflow`; §B B9 |
| FR-014 follows the location, same refresh | follow step, one `Changed` re-reads both, R6 | integration test through `State::update` (H7); §B B2, B7 |
| FR-015 docs, snapshot, visual pass | Docs above, R14 | docs gate (`check-user-guide-updated.sh`); `layout_snapshot`; quickstart §B |

## Delivery order

1. **Rules** (no GUI): `context_header`, `StatusPhrase::of_status`; `features::session_strip` with the
   moved predicates (the bar keeps working from the moved functions); `HeaderCounts`, follow step,
   `ReadCounts` shell half, watch, `request_titles`; `location_card`.
2. **Components**: `Tag` extensions, `tab::SESSION_WIDTH`, `cdk::Keyed`, keymap label, showcase.
3. **Session area**: `ui/session_area.rs`, `ui/mod.rs` branch, bottom bar removed from
   `terminal::pane`; stability tests, gates and covered states migrated; fixture regenerated.
4. **Docs and visual pass**.

Parallel work: #1124 (status bar) and #1125 (sidebar) also change the main window. This plan touches
`ui/mod.rs` only inside the main area's active-session and Changes branches, not the window column
or the drawer, and reads the sidebar's order through `features::navigation::location_node`, so a
rework of the sidebar's rows keeps the strip's order as long as `unfolded_sessions` stays.

## Project Structure

### Documentation (this feature)

```text
specs/1126-session-context-header/
├── spec.md
├── plan.md               # this file
├── research.md           # R1–R14
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── context-header.md # header text rules, follow, press, layout (H1–H10)
│   └── session-area.md   # strip rules, layout, stability, fit, order (S1–S9, A1–A2)
└── tasks.md              # /speckit-tasks
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── lib.rs                        # + pub mod context_header;
├── context_header.rs             # new
└── pr_list.rs                    # StatusPhrase::of_status
crates/micold-client/src/
├── app.rs                        # follow step
├── keymap.rs                     # new_terminal_chord_label
├── features/
│   ├── mod.rs                    # + pub mod session_strip;
│   ├── session_strip.rs          # new (predicates moved in from ui/terminal.rs)
│   ├── session.rs                # StripTab from session_strip
│   └── changes.rs                # HeaderCounts, HeaderFollowed, HeaderCountsRead, ReadCounts
├── shell/{changes.rs,issues.rs,subscriptions.rs}
├── ui/
│   ├── mod.rs                    # active-session branch
│   ├── session_area.rs           # new
│   ├── terminal.rs               # bottom bar removed; re-exports session_strip
│   ├── overview.rs               # location_card
│   ├── cdk/{mod.rs,keyed.rs}     # Keyed promoted from ui/transcript/keyed.rs
│   ├── transcript/{mod.rs,keyed.rs}  # keyed.rs removed, uses cdk::Keyed
│   └── material/{tag.rs,tab.rs}
└── showcase/sections/            # session tab, header chips
crates/micold-client/tests/
├── session_area_stability.rs     # replaces terminal_bar_stability.rs
├── terminal_tabs.rs, tab_reveal.rs, features_changes.rs
├── gates/{strip_controls_hold_their_size.rs,strip_controls_anchor_the_trailing_edge.rs,
│          session_area_fits.rs,tab_children_fit.rs,containment.rs}
├── support/covered_states.rs
└── fixtures/layout_snapshot.txt
docs/user-guide/{terminal-panes.md,reviewing-changes.md}
docs/development/layout-snapshot.md
```

**Structure Decision**: Existing workspace layout; text rules in `micold-core`, strip and counts rules
in `micold-client`'s render-free `features/`, rendering in `ui/session_area.rs` with shared
components in `ui/material` and `ui/cdk`.

## Complexity Tracking

None.
