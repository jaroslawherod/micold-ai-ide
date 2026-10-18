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
- **Next step**: M2: full gate, visual pass, push, PR title/body to scratchpad; then M3

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1034 | Design: spec, plan, research, contracts, tasks | merged | 1fae1becb8fa64097695f169ad7243ec5d1f1dae |
| #1042 | M1: card context menu (T001–T013) | merged | 71bf8848af5fc1ca0f5fc8e8e167778b753da43a |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Right-click a card: *Open* + row-menu entries, run as the row menu does; registry dismissal | #1042 | merged |
| M2 | T014–T019 (T016b) | light | Target mark on the card; empty-space right-click dismisses | — | implemented, reviewed; full gate and visual pass pending |
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
| A M1 (code-review high) | 1 | c81d74a8449b3e036a2e8ffe80aa60e5168c827e:5dbd5098f5e91b3d5aae7afdbb0508cf50921e8c | CLEAN: 3 MINOR not fixed (settle_card_menu in State::update, clone in CardMenuEntryChosen, hand-copied MenuItem in with_message) |
| B M1 (conformance, sonnet) | 1 | c81d74a8449b3e036a2e8ffe80aa60e5168c827e:5dbd5098f5e91b3d5aae7afdbb0508cf50921e8c | CLEAN; Verify: 8 test binaries ok, 144 passed, 0 failed |
| Visual pass M1 | 1 | c81d74a8449b3e036a2e8ffe80aa60e5168c827e:5dbd5098f5e91b3d5aae7afdbb0508cf50921e8c | B1, B3–B9, B16 PASS; B14, B19 PARTIAL (fixture: no session / seeded unbound daemon); B15 not run (no second window); record in visual-pass.md |
| Rebase M1 onto main 1927ab5c (sonnet, conflict resolutions) | 1 | 5531061f2042f54caf19e95871eb69221b0e4d96:faed02ae760714e5641773993f2d836b81f2fe7e | CLEAN: 2 MINOR not fixed (module doc line length, pre-existing; CardMenuEntryChosen during activation unreachable and due() filters on open) |
| A M2 (reviewer, high) | 1 | 04ef833dcb2e901f6ca01478ff005826cab0ab4c:71bf8848af5fc1ca0f5fc8e8e167778b753da43a | CHANGES: F1 BLOCKER (outer `ContextArea` also dismissed the card's fresh menu) fixed in `ContextArea::update` (skips when captured) + widget test; F2 MAJOR (no test of pane dismissal) fixed by same test; F3 MINOR declined |
| B M2 (conformance, sonnet) | 1 | 04ef833dcb2e901f6ca01478ff005826cab0ab4c:71bf8848af5fc1ca0f5fc8e8e167778b753da43a | CHANGES: F1 MAJOR (cycle-log rows missing) fixed; F2 MINOR declined, F3 MINOR fixed |

## Declined review findings

- A M2 F3 (reorder fixture convoluted), B M2 F2 (targeted vs hovered render): the harness draws with `Cursor::Unavailable`, so a hovered render cannot be expressed; hover is covered by the 971 ripple tests. A F3 is taste.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Review A M1 MINORs left (taste, no behaviour change): `settle_card_menu` called from `State::update` (app.rs:1041); `inner.clone()` for `CardMenuEntryChosen` (main.rs:665); `with_message` copies `MenuItem` fields by hand (ui/overview.rs:661) — a `MenuItem::map_message` would be sturdier.
- Visual pass: the worktree Rename dialog does not take keyboard focus on open (pre-existing on every path: `rename_started` sets no focus); over the Overview, typed Return can reach the still-focused card behind the dialog. Seen once in B3. Candidate for M3 (focus handling) or its own issue.
