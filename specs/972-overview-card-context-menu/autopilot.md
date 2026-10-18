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
- **Next step**: milestone M1, continue at T006 (see Handover)

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

M1 unit 1 handed over at 160k context. Branch `claude/project-thread-a1yb7j` (no PR yet; do not run
`branch-start.sh` to reset: the commit below is unmerged M1 work, it rebases onto main).

- Done: T001–T003 (ticked). `Icon::Open` (`login`, U+EA77) + docs/user-guide/icons.md row;
  `ui::location_menu_items(state, &SessionLocation)` extracted, the worktree row menu calls it.
- T004 + T005 tests written in `crates/micold-client/tests/overview_card_menu.rs` (one file, both
  parts) and committed **red** (types missing; evidence in tdd/cycle-log.md). Not ticked: they
  tick with T009/T010 green. It uses `ui::overview::{card_menu_items, cards}`,
  `features::overview::{update, CardMenu, MenuOrigin, Msg, OverviewCardMenu, State}`,
  `features::Outcome`; `CardMenu` must derive `Debug, Clone, PartialEq` (compared with `assert_eq!`).
- Next: T006 (displacement/registry/registration tests), T007, T008, then T009–T013.
- Findings for T009: `session::menu_toggled` and `start_menu_toggled` report no
  `SurfaceOpened`, so for R2's "session menu / start menu displace the card menu" they must return
  `surface_opened(.., ID)` (give `SessionContextMenu` an `ID` const) and the registry gains
  `displaces: OverviewCardMenu` on them; `tests/popover_displacement.rs` uses `State::default()`,
  but a card menu survives `settle_card_menu` only with the Overview open and its target listed
  (a project with a Default card): give the pair test a base state with a project and
  `overview.open = true`, and add `overview_card_menu` to DISPLACES and the opener. The
  displacement table changes for five rows (project/worktree/run_group/session/session_start
  each gain `overview_card_menu`).
- `Msg::ProjectSwitched` must also clear `card_menu`/`focus_return` (test expects it; the
  Overview stays open on a switch). `GridScrolled`: app.rs `Message::Overview` arm calls
  `dismiss_on_scroll_beneath()`. settle at the outermost epilogue beside `agent_confirm::release`.
- Running tests: a foreground Bash may wait 250 s; start cargo detached and `hold.sh`.

## Open escalation

None.

## Follow-ups not done

None.
