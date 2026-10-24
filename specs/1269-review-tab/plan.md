# Implementation Plan: Review as a per-worktree tab that keeps its state

**Branch**: `claude/project-thread-6rh2yo` | **Date**: 2026-10-24 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1269-review-tab/spec.md`

## Summary

The Changes view stops being a mode that replaces the terminal and is forgotten on any switch. Each
location gets at most one **Review** tab in its session tab strip (feature 1126), holding a kept view
until its × or its worktree's removal. The rules — the open-tabs map keyed by `(project, location)`,
which Review tab is shown, open-or-show, hide, close and which session to show after, the drop rules,
the "changed since you looked" freshness state, the last-shown session per location, and the words of
the tab and the Changes button — are a new generic, render-free core module
`micold_core::review_tabs` (`ReviewTabs<V>`), unit-tested with no window and shippable alone (M1).
The client then replaces `features::changes::State.open: Option<OpenView>` with
`ReviewTabs<OpenView>`, routes every view message to the shown view, turns session selection into a
hide, adds × (`Msg::TabClosed`), makes the follow step follow a derived `shown_location()` (so a
sessionless worktree's review has its own header), and swaps every `changes.open.is_some()` focus and
attention predicate for `review_shown()` (M2). Last, the strip gains a `Member::Review` with a draft
badge (new shared `CountBadge`), the changed dot (`UnreadMark`) and ×, replacing the view's **Close**
(M3). Nothing depends on #1125 (sidebar) or #1127 (title bar). Decisions: [research.md](research.md).

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (existing `material::Tab`, `cdk::Keyed`); notify (existing
watch); no new crate

**Storage**: none. Tabs and kept views live in window memory only (D2); comments stay where they are
(service-owned, unchanged); nothing persisted or sent (Principle IV)

**Testing**: `micold-core` unit tests for `review_tabs` (T1–T8, incl. a property test of the drop
rule); client lib tests for `features::changes` (routing, hidden reads, changed mark),
`features::session_strip` (members, presses, widths), `ui::session_area::changes_press`, `app`
readers; client integration tests through `State::update` (`features_changes.rs`,
`terminal_focus.rs`, `attention_view_report.rs`, `keyboard.rs`, `navigation.rs`,
`idle_subscriptions.rs`, `session_area_stability.rs`); geometry gates and covered states in
`tests/layout_snapshot.rs`; visual pass (quickstart §B). Whole gate: `scripts/gate.sh`

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: no read for a hidden tab (SC-003); exactly one list read when a marked or
unwatched tab is shown; strip derived per render, O(sessions + shells + 1)

**Constraints**: press acts on first try while badge, mark or tabs change; no overlap at 640×480 or
1280×800; pane chords and focus unchanged; no new watcher (only the shown location is watched)

**Scale/Scope**: core: 1 new module (`review_tabs.rs`). Client: `features/changes.rs` (state, ~89
`state.open` uses → `state.tabs.shown_mut()`/`shown()`, `Opened`/`TabClosed`/`SessionSelected`/
`Changed`/`ReviewChanged`/`WorktreesListed` arms), `app.rs` (readers, follow step,
`ChangesRequested`, focus and attention predicates), `features/session.rs` (`closed`, R8),
`features/project.rs` (`ForgetConfirmed`), `shell/{changes,pr_status,subscriptions}.rs` (readers of
`open`), `ui/{mod,session_area,changes,overview,status_bar}.rs`, `features/session_strip.rs`,
`ui/material/{tab,count_badge (new),section_list,mod}.rs`, `showcase/`; tests and gates; docs: three
user-guide pages and the layout-snapshot page

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every rule with a branch is red-first: tab rules, freshness, drop rules, last-shown session and faces in `micold_core::review_tabs` (T1–T8); routing, hidden reads, changed mark in render-free `features::changes` (U1–U6); follow step, entry points, ×, last-session rule, focus and attention through `State::update` (U3, U4, U7–U10); strip members, presses and widths in render-free `features::session_strip` (U13–U15). The strip and header widgets only lay these out (Principle I glue exception), checked by geometry gates and quickstart §B. |
| II. Multi-Session | PASS | Tabs are per location, shared by its sessions; selecting between them keeps the tab (edge case *Several sessions*); a session's stopped state never marks the Review member (it has no process). Each window owns its `ReviewTabs` (window memory); comments, and so the badge, stay shared as today (edge case *Several windows*). Sessions' own persistence is untouched. |
| III. Worktree Integration | PASS | A tab is keyed by the project's Default entry or a managed worktree's `dir_name`; reads are the view's existing worktree-aware reads; a removed worktree drops its tab in every window (`WorktreesReplaced`, T4). No worktree is created, changed or removed. |
| IV. Local-First | PASS | No new network call or storage; tabs are in memory; the pull request source keeps today's reads. |
| V. Rust + iced | PASS | iced only. `shown: Option<EntryKey>` naming a key of the map makes "one shown Review tab per window" structural; `badge: Option<NonZeroUsize>` makes a `0` badge unrepresentable; `Freshness` is a closed enum, so the mark is a total function (T8); `MemberKey::Review` keeps the keyed row's key `Copy`. |
| VI. Cross-Platform | PASS | No platform code; tooltips naming a chord take it from `keymap` per platform; fonts bundled, so gates measure what each platform draws; CI on three platforms. |
| VII. Documentation | PASS | `docs/user-guide/reviewing-changes.md` in M2 and M3, `terminal-panes.md` and `worktrees-and-sessions.md` in M3, `docs/development/layout-snapshot.md` *What is covered* in M3 (FR-017, R14). |
| VIII. Components | PASS | The Review member is the shared `material::Tab` with `.trailing`; the changed mark is the shared `UnreadMark`; the draft count is a new shared builder `material::CountBadge` (promoted, the section list's badge chip moves onto it) shown in the showcase; all builders end in `.into()`, colours from `Roles`. |

Post-design re-check: unchanged, all PASS. No Complexity Tracking entries.

## Design

### Core (`crates/micold-core`)

- **`src/review_tabs.rs`** (new; `pub mod review_tabs` in `src/lib.rs`): `EntryKey`, `Freshness`,
  `ReviewTab<V>`, `ReviewTabs<V>` with `open_or_show`, `hide`, `close`, `drop_gone_worktrees`,
  `drop_project`, `files_changed`, `location_left`, `comments_changed`, `uncovered`, `remember`,
  `session_for`, `show_if_present` and readers; `Opened`, `Closed<V>`, `Change`; `ReviewTabFace`;
  `ChangesButton`, `Press`. Contract: [contracts/review-tabs.md](contracts/review-tabs.md) T1–T8;
  data: [data-model.md](data-model.md) §Core. Uses only `std` and `micold_core::session::{SessionId,
  SessionLocation}`.

### Client (`crates/micold-client`)

- **`src/features/changes.rs`**: `State.tabs: ReviewTabs<OpenView>` replaces `open` (R6);
  `Msg::Opened` → `open_or_show`, building today's fresh `OpenView` only on `Created`, settling
  in-flight loads before a `reread` (U5); `Msg::Closed` → `Msg::TabClosed { project, entry }` (U4);
  `Msg::SessionSelected` → `hide` (U2); `Msg::Changed` also marks a hidden tab of the header's
  location (U6); `Msg::ReviewChanged` → `comments_changed`; `Msg::WorktreesListed` →
  `drop_gone_worktrees` (U9). Every other arm reads `state.tabs.shown_mut()`.
- **`src/app.rs`**: readers `review_shown`, `shown_location`, `review_watch_entry`,
  `review_covered`; `follow_header` follows `shown_location()` and feeds `remember`,
  `location_left`, `hide` (other project), covered → `location_left`, `uncovered` (U7, R5);
  `Outcome::ChangesRequested` switches location through `session_for` + `set_current_session`
  before `Opened` (U3, R9); `Outcome::ChangesClosedForCompare` hides; the × follow-up
  (`set_current_session(show)`, `focus_terminal`) (U4); `terminal_focused`, `view_facts`,
  `header_watch_entry` use the shown review (U10, U11, R12).
- **`src/features/session.rs`**: `closed` shows the location's Review tab when its last session
  closed (U8, R8).
- **`src/features/project.rs`**: `ForgetConfirmed` → `drop_project` (U9).
- **`src/shell/changes.rs`, `src/shell/pr_status.rs`**: read `tabs.shown()` where they read `open`.
- **`src/shell/subscriptions.rs`**: the view watch from `review_watch_entry()` (U11, R13).
- **`src/ui/mod.rs`**: `session_view_visible`, `main_content_key` and the main-area branch use
  `review_shown()`; the Review branch draws `session_area::over_review` (U10, U16).
- **`src/ui/session_area.rs`**: `changes_press` and `changes_button` from `ChangesButton::of`
  (U12); `over_changes` → `over_review`: the header of `shown_location()` above the view in M2,
  plus the strip in M3 (U16).
- **`src/ui/changes.rs`**: the toolbar's **Close** sends `TabClosed` (M2), then is removed (M3, U13).
- **`src/features/session_strip.rs`** (M3): `Member::Review`, `MemberKey::Review`, `StripModel::of`
  from `shown_location()` incl. sessionless, marked rule extended, presses, `width` (U13–U15, R10).
- **`src/ui/material/count_badge.rs`** (new, M3), **`tab.rs`** (`REVIEW_WIDTH`), **`section_list.rs`**
  (badge chip on `CountBadge`), **`mod.rs`** (export); **`src/showcase/`** (the Review tab hidden with
  badge and dot, `CountBadge`) (R11).
- **`src/ui/overview.rs`, `src/ui/status_bar.rs`** tests: `state.changes.open` → `tabs`.

### Tests and gates (`crates/micold-client/tests`)

- `features_changes.rs`: `selecting_a_session_closes_the_view` becomes kept-and-hidden (U2); new U1,
  U3, U4, U7–U9 cases. `features_pr_changes.rs`, `features_settings.rs`, `features_sidebar.rs`,
  `app_state.rs`, `navigation_apply.rs`, `overlay_dismissal_delta.rs`, `support/modals.rs` migrate
  `open` reads.
- `terminal_focus.rs`, `attention_view_report.rs`, `keyboard.rs`, `navigation.rs`,
  `idle_subscriptions.rs` (U10, U11).
- `session_area_stability.rs`: the Changes-press case asserts no `Closed`; new Review press and ×
  cases (U12, U14).
- Gates `session_area_fits.rs`, `tab_children_fit.rs`, `strip_controls_hold_their_size.rs` with the
  Review member (U15); `support/covered_states.rs`: `session-area-review-shown`,
  `session-area-review-hidden`, `session-area-review-sessionless` replace
  `session-area-changes-open`; `fixtures/layout_snapshot.txt` regenerated in M3 only.

### Docs

`docs/user-guide/reviewing-changes.md` (*Opening the Changes view*, *Keeping up with changes*: M2
kept view and re-read on show; M3 the tab, badge, changed mark, ×), `docs/user-guide/terminal-panes.md`
(the strip's Review tab; chords with it shown), `docs/user-guide/worktrees-and-sessions.md` (where it
describes the strip), `docs/development/layout-snapshot.md` (*What is covered*).

## Requirement map

| FR | Where | Test layer |
|---|---|---|
| FR-001 tab in the location's strip, header and strip above | `Member::Review` last member (U13), `over_review` (U16), R10 | client unit `session_strip`; covered states; §B B1 |
| FR-002 at most one per location | `ReviewTabs` map, T1 | core unit (repeated opens); integration U3 (SC-005) |
| FR-003 button and Review changes create or show; other location becomes shown | T1, T6, `ChangesRequested` (U3, R9) | core unit; integration U3; §B B7, B8 |
| FR-004 session/shell tab press keeps the Review tab | `hide` T2, U2 | core unit; integration U2 |
| FR-005 switches, Overview, Settings keep tabs | T2, T4 property, follow step U7 | core property test; integration U2, U7 (SC-001, SC-002) |
| FR-006 kept view presented as left | `OpenView` kept in the map, U1; re-read keeps parts as today | integration U1 (whole `OpenView` equality); §B B3, B6 |
| FR-007 × removes, shows previous else main, keeps comments and PR | T3, U4 | core unit; integration U4; §B B10 |
| FR-008 worktree removal drops in every window | T4 `drop_gone_worktrees`, U9 via `WorktreesReplaced` | core unit; integration U9 |
| FR-009 draft badge, none at zero, follows any window | `ReviewTabFace` T8 from `reviews` (U13) | core unit; client unit `session_strip`; §B B5 |
| FR-010 no read while hidden; changed mark | T5, U5, U6, U11 | core unit; client unit effect counts (SC-003); §B B5 |
| FR-011 re-read on show when marked or unwatched | T5 `reread`, U5 settle + one read, U7 `location_left` | core unit; client unit; integration U7; §B B6 |
| FR-012 button selected while shown; press changes nothing | `ChangesButton` T8, `AlreadyShown` T1, U12 | core unit; client unit `changes_press`; §B B1, B4 |
| FR-013 chords and focus unchanged | `review_shown()` in the same predicates (R12), session chords hide (U2) | `terminal_focus.rs`, `keyboard.rs`, `navigation.rs`, `app_chord_tests` (SC-004); §B B12 |
| FR-014 in-view decided as today | `view_facts` (U10) | `attention_view_report.rs` |
| FR-015 first-try presses; no overlap at minimum width | `MemberKey::Review`, fixed `REVIEW_WIDTH` (U14, U15) | `session_area_stability.rs`; gates at 640×480 and 1280×800 (SC-006); §B B11 |
| FR-016 rules testable without a window | `micold_core::review_tabs` (R1) | `mise run test-core` |
| FR-017 docs, snapshot, visual pass | Docs above, R14 | docs gate; `layout_snapshot`; quickstart §B |

## Delivery order

Each milestone is one PR, shippable alone; M1 and M2 touch nothing of #1125's sidebar rows or
#1127's title bar (the sidebar's existing **Review changes** message is reused as is).

1. **M1 — Core rules** (no client change): `micold_core::review_tabs` with T1–T8 tests. Merges
   while #1125 and #1127 are open; nothing calls it yet (public API of the core crate, so no
   dead-code lint).
2. **M2 — Kept views** (client state, no new widget): `ReviewTabs<OpenView>` in `features::changes`,
   routing and hidden reads, `SessionSelected` hides, `TabClosed` behind the existing **Close**,
   `shown_location()` and the follow step, entry points incl. another and sessionless locations
   (drawn under their own header), last session closed, removal, focus/attention/watch predicates,
   the Changes button's selected state and tooltip; `reviewing-changes.md` for the kept view. The
   user-visible change: switching away and pressing **Changes** brings the review back as left.
3. **M3 — The tab**: `CountBadge`, `tab::REVIEW_WIDTH`, `Member::Review` (incl. the sessionless
   strip), × replaces **Close**, gates, covered states and fixture, showcase, the rest of the docs,
   visual pass.

Parallel work: #1125 and #1127 change the window's sidebar and title bar. This plan touches
`ui/mod.rs` only in the main area's Changes/session branches and `session_view_visible` /
`main_content_key`, and reads the location's sessions through
`features::navigation::location_node` as `StripModel::of` already does.

## Project Structure

### Documentation (this feature)

```text
specs/1269-review-tab/
├── spec.md
├── plan.md               # this file
├── research.md           # R1–R14
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── review-tabs.md    # core rules T1–T8
│   └── review-tab-ui.md  # client state, routing, strip, layout U1–U16
└── tasks.md              # /speckit-tasks
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── lib.rs                          # + pub mod review_tabs;
└── review_tabs.rs                  # new (M1)

crates/micold-client/src/
├── app.rs                          # readers, follow step, ChangesRequested, × follow-up, predicates (M2)
├── features/
│   ├── changes.rs                  # State.tabs, Msg changes, routing (M2)
│   ├── session.rs                  # closed: last session of a location (M2)
│   ├── project.rs                  # ForgetConfirmed drops tabs (M2)
│   └── session_strip.rs            # Member::Review, StripModel from shown_location (M3)
├── shell/{changes,pr_status,subscriptions}.rs   # readers of the shown view, watch (M2)
├── ui/
│   ├── mod.rs                      # review_shown in branches and keys (M2)
│   ├── session_area.rs             # changes_press, over_review, strip member (M2, M3)
│   ├── changes.rs                  # Close → TabClosed (M2), removed (M3)
│   └── material/{count_badge.rs (new), tab.rs, section_list.rs, mod.rs}   # (M3)
└── showcase/                       # Review tab, CountBadge (M3)

crates/micold-client/tests/         # see Tests and gates
docs/user-guide/{reviewing-changes,terminal-panes,worktrees-and-sessions}.md
docs/development/layout-snapshot.md
```

**Structure Decision**: the existing three-crate workspace; rules in `micold-core`, state and
routing in the client's render-free modules, widgets in `ui/`.

## Complexity Tracking

No violations.
