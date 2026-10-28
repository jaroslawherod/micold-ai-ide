# Contract: the history's behaviour (UI)

**Feature**: 1465 | [plan](../plan.md) | [data model](../data-model.md) |
[research R1–R6, R9, R11](../research.md)

## U1 — Recording

After every message the binary's `update` calls `State::observe_place()`. With `history.moving`
unset, a shown place (research R3) different from the current entry is recorded; an equal one, or
none, records nothing. Covered moves: sidebar click, tab strip, pane focus moving to another
session, navigation chords, quick switcher, notification, opening or creating a session, opening
the Overview, Compare or a Review tab, and the app's own moves (a shown session ending).

## U2 — A press

`shell/history.rs::update(app, Msg::Pressed { dir, source })`:

0. `app.core.workspace.refresh_availability(app.caps.scanner())`, so `exists` and the switch read
   the same availability (research R5).
1. `step = app.core.press_history(msg, &lookup)` (the root drives the feature) with
   `lookup = |project| Some(discover_worktrees(git, project) dir_names)`; `exists` adds the
   project's last list `worktree.known[project]`, and `resolve` reads `lookup` once per project.
2. `follow_ups = app.core.apply_history(step)`; each runs through the root `update`.
3. `app.core.finish_history_move()` (only for `Step::Show`).

No decision of its own: which messages, and whether to switch project, is `apply_history`'s.

## U3 — Resolution (`features::history::resolve`)

| Condition (first that holds) | Step |
|---|---|
| Settings open or a registered surface open (`navigation::blocked`) | `None` (FR-010) |
| `source == Key` and a terminal is focused or `live_focused_field` is some | `None` (FR-009) |
| `Back`, the shown place is a Review tab, its view `can_go_back` | `ReviewBack` (FR-011) |
| `History::step` finds a target | `Show(target)`, `moving = true` (FR-006) |
| otherwise | `None`, no error, no notification (FR-006, SC-002) |

## U4 — Showing a target (`State::apply_history`)

| Target view | Messages, in order (prefixed with `Project(Reopened(project))` when `project` is not the active one) |
|---|---|
| `Session { id, .. }` | `Session(Selected(id))` |
| `Review(location)` | `Overview(Closed)` if open, `Runs(CompareClosed)` if open, `Sidebar(ReviewChangesRequested(location))` |
| `Overview` | `Runs(CompareClosed)` if open, `Overview(Opened { settings_open: false })` |
| `Compare(group)` | `Overview(Closed)` if open, `Runs(CompareOpened { group, project })`; in another project the group is found in `runs.known[project]` (research R5) |

`ReviewBack` → `Changes(BrowseBack)`. A side-button move lands the keyboard as a click on that place
leaves it (spec *Assumptions*); no `finish_navigation`. A key move (`Source::Key`) then gives the
keyboard back to the app (`State::finish_history_move(source)` releases a terminal the move focused),
so the next press or repeat steps again (US2 AS4, FR-012) rather than reaching that terminal
(M2, ledger decision 6).

## U5 — Gone targets

`exists` (research R5) is decided before any message is sent, so a press that finds nothing changes
nothing, and a press never switches project towards a gone target. Gone entries passed over are
removed from the list (FR-007).

## U6 — The Review sheet

`changes::sheet_model(single_keys)` appends, after the `REVIEW_KEYS` rows, `(history_key_label(Back),
"Back (also the mouse back button)")` and `(history_key_label(Forward), "Forward (also the mouse
forward button)")`, with `single_keys` on and off (FR-014). The sheet view and the showcase poses
are unchanged code.

## U7 — Tests

`crates/micold-core/src/history.rs` (unit, `mise run test-core`):
- `record`: equal to current adds nothing; after Back, recording drops the forward entries (US1
  AS3); 60 records keep 50, the oldest dropped, Back stops at the oldest kept (SC-004).
- `step`: Back/Forward at the ends → `None`, position unchanged (US1 AS6); A→B→C with B gone, Back
  from C → A and B removed (US4 AS1); all behind gone → `None` and they are removed (AS3); A→B→A→C
  with B gone: removal merges the neighbouring As, Back from C lands on A once; an entry equal to
  `shown` is passed over, never returned (AS4); `step` never records.
- SC-002 sweep: for every subset of gone entries in a five-entry list and every position, each
  `step` returns an existing entry different from `shown`, or `None`.

`crates/micold-client/tests/history.rs` (new, root `State`, no window):
- U1 for each mover (sidebar `Selected`, quick switcher choice, navigation chord step, Overview
  `Toggled`, `CompareOpened`, `ReviewChangesRequested`, a shown session's removal moving the shown
  place); background `Started` / catalog updates of other sessions record nothing; re-selecting the
  shown session records nothing (FR-003).
- U3 rows, each: Settings, a dialog, the quick switcher, the Review sheet → `None`; `Key` with a
  live field, with a stale `ReviewComposer`, with a focused terminal; `Button` with a live field →
  acts (FR-008).
- FR-011: Review tab with a browse frame → `ReviewBack` and the position unchanged; without →
  global Back; Forward from a Review tab → global.
- U5: a session archived, a worktree removed from `worktree.worktrees`, a worktree missing from the
  injected `lookup` of another project, a project turned unavailable → skipped and removed.
- U4: `apply_history` messages for each target view, same and other project.
- Cross-project Compare: in P record Compare(G), switch to Q, Back → P active, Compare of G open,
  the forward entries kept (FR-006, FR-007); a Compare entry whose group is gone from
  `known[P]` is skipped and removed; `known` keeps each project's last `GroupsChanged` list.
- `observe_place` while `moving` records nothing; `finish_history_move` records only a place
  different from the target.
- Two `State`s keep independent histories.

`crates/micold-client/src/main_tests.rs` (binary, real root `update`):
- A → B → C, Back, Back, select D: forward is empty (US1 AS3) and the entries are A, D.
- Cross-project Back: the project switches, the target session is shown, the entries and position
  are as before the press except the position (R2: no intermediate place recorded).
- Overview → Back → the previous session, Overview closed; Back from Compare to a Review tab.
- Cross-project Compare: Compare of G in P, show Q1, Back → P with Compare of G open, Forward → Q1.

`crates/micold-client/tests/review_keys.rs`: `sheet_model(true)` and `sheet_model(false)` end with
the two U6 rows.
