# Autopilot ledger — #972 overview-card-context-menu

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Overview: right-click context menu on cards (#972). Overview cards only respond to a primary click, so actions on a worktree/session need opening it first. Right-clicking a card opens a context menu at the pointer with the card's actions (e.g. open, rename, delete). It is also reachable from the keyboard (Menu key / Shift+F10) on the focused card, dismisses on Esc or outside click, and the right-clicked card shows as the target. Alternatives: per-card overflow button only — discoverable but clutters every card. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #972
- **Worktree branch**: claude/project-thread-a1yb7j
- **Started**: 2026-10-18
- **Phase**: implement
- **Next step**: milestone M1, compile WIP, then T010 (see Handover)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1034 | Design: spec, plan, research, contracts, tasks | merged | 1fae1becb8fa64097695f169ad7243ec5d1f1dae |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Right-click a card: *Open* + row-menu entries, run as the row menu does; registry dismissal | — | in progress |
| M2 | T014–T019 (T016b) | light | Target mark on the card; empty-space right-click dismisses | — | pending |
| M3 | T020–T028 | full | Menu key / Shift+F10 open, arrow/Enter navigation, focus return | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | Open card menu when the grid scrolls beneath it? | Closes, no entry runs (FR-012) | agent | specs/017-material-component-architecture/spec.md#FR-009 |
| 2 | clarify | Focus after an entry runs from a keyboard-opened menu? | Target card if still listed, else first card (FR-011) | agent | spec FR-011; specs/813-worktree-overview/spec.md#FR-014 |
| 3 | tasks | US1 has 11 tasks (T003–T013 incl. foundational refactor): split? | Kept whole: every US1 scenario needs the menu open and its entries running; no half has its own deliverable | agent | tasks.md M1 |
| 4 | tasks | speckit-analyze findings | Fixed in artifacts: FR-013 scoped to no open menu (spec), plan builder count and tree, quickstart B18/B19, T016 split (T016b), T021 key capture | agent | analyze report |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | df9363fdbdc355faf73bcb348d3b00d3d5979cbd:8ee4531564743d6032729cd056a73f655abfb1a0 | CLEAN: 3 MINOR (F1, F2 fixed, prose/edge cases; F3 left: keyboard nav already stated for both open paths in US3.4) |
| Plan | 1 | ddc1f4f2a559acde12172eb355263991ae37114a:fe5fce887724a3fa7f184edd604ecb3f472b830a | CLEAN |
| Tasks | 1 | c3044e31781aae114ef17bc23ba9864d7455f4e7:fe73a9896b90d8e7dfd5ee8ec9c76a9e4daaa49e | CHANGES: F1 MAJOR (keyboard reducer effects in M1, tested only in M3) fixed: assertions moved T023→T004 |
| Tasks | 2 | 7e95c7a89d11a89ad961016242462c7b04b0e816:fe73a9896b90d8e7dfd5ee8ec9c76a9e4daaa49e | CLEAN (sonnet) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 unit 2 handed over at 150k context. Branch `claude/project-thread-a1yb7j` (no PR yet; do not run
`branch-start.sh` to reset: the commits are unmerged M1 work). The last commit is **WIP, not yet
compiled**: build it first (`cargo test -p micold-client --test overview_card_menu --test
popover_displacement --test overlay_registry --test overlay_registration --test layout_snapshot
--lib --no-run`, detached) and fix what it reports.
- Done before: T001–T003 (ticked); T004/T005 tests red in `tests/overview_card_menu.rs`.
- This unit, tests written (red evidence in tdd/cycle-log.md), not ticked:
  T006 (`popover_displacement.rs`: `overview_card_menu` row + five rows gaining it, `base()` state
  with a project and the Overview open, opener; `overlay_registration.rs`: POPOVERS row + band /
  cancel test; `overlay_registry.rs`: escape + scroll_beneath test), T007 (`overview_card.rs`
  secondary-press test, `card_grid.rs` wheel test, `cards<M>` fixture made generic), T008
  (`gates/context_menu_anchor.rs::the_card_menu_stays_inside_the_window_at_its_edges`, uses the
  new `ui::menu_panel_size` re-export).
- T009 implemented (uncompiled): `features/overview.rs` `MenuOrigin`, `CardMenu`, `State::{card_menu,
  focus_return}`, `Msg::{CardMenuOpened, CardMenuDismissed, CardMenuEntryChosen, GridScrolled}`,
  `OverviewCardMenu` surface + `Registered`; registry displacement lists; `SessionContextMenu::ID`,
  `SessionStartMenu::ID`; `session::menu_toggled` returns `surface_opened`; the `StartMenuOpened`
  arm prepends `SurfaceOpened(SessionStartMenu::ID)` to the notice; app.rs Overview arm calls
  `dismiss_on_scroll_beneath()` after `GridScrolled`.
- Next: compile + T004 reducer/T006 green → tick T006, T009; then T010 (`ui::overview::card_menu_items`
  + `settle_card_menu`, called in app.rs epilogue before `agent_confirm::release` inside
  `update_depth == 0`), T011 (`OverviewCard::on_secondary_press` via `cdk::context_area::ContextArea`,
  `CardGrid::on_scroll` via `Scrollable::on_scroll`), T012 wiring (ui/overview.rs view, ui/mod.rs
  `view_with` push `card_menu` surface beside `session_menu`, main.rs `overview_message`
  `CardMenuEntryChosen(inner)` → reducer then `update_inner(app, *inner)`), T013 docs; then verify.md.

## Open escalation

None.

## Follow-ups not done

None.
