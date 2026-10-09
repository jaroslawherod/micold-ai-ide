# Autopilot ledger — #484 terminal-split-panes

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 484 — issue #484 "Split the terminal area into panes to watch several sessions at once" (scope verbatim in the issue); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #484
- **Worktree branch**: claude/project-thread-wysm57
- **Started**: 2026-10-09
- **Phase**: milestone M1
- **Next step**: M1 in progress (implement)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #655 | Design (spec, plan, tasks) | merged | 909f4b233cfe660881ab0cbdc62a0ad7488e5221 |
| #654 | Previous run (#483), not this run's | merged | 334cc99ff48ae82e60a4a107f4b76cb1ef9d5959 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T020, T006a | full | Live panes with focus and own PTY size (US1+US2+US5 folded: no half-wired UI; diff exceeds split threshold but no scenario split leaves a working deliverable) | — | pending |
| M2 | T021–T023 | full | Pane shortcuts (split, focus) | — | pending |
| M3 | T024–T028 | full | Resize, close, rearrange | — | pending |
| M4 | T029–T035 | full | Layout survives restart | — | pending |

## Decisions

- Clarify round 1: 3 markers agent-resolved (one pane per terminal; empty panes persist; chords Ctrl/Cmd+Shift+D/H/W/Arrow, not Ctrl+Alt+Arrow). No escalation.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 303c12f166730835012ddb99e2d6f068115ff086:e61d1682ef7b666342bfb8834c130312e9e3e26a | CHANGES: 1 MAJOR (bad `feature 182` citation: it was a commit hash; fixed, prose only), 3 MINOR (fixed, prose only) |
| Plan | 1 | dbb88a3c2cb206cb6716902bb89d94ac49cd8803:8f919183081da946845bf3a7e9b909967d5192fe | CHANGES: 2 MAJOR (layout write path: daemon-owned via Workspace + SetPaneLayout; process field semantics), 4 MINOR; fixed |
| Plan | 2 | 2c92523f4feb02154bd127d01e88edcb54c2d30c:ea4f3c2260e6d692e94b4d3122dfc2776902dafb | CLEAN (1 MINOR: snapshot name, fixed, prose only) |
| Tasks | 1 | 0301e32716201fec8325e22c1d5d8d31d669aebd:5a7cd75a2dbce27d266c19362ff9763f1b66a99a | CHANGES: 2 MAJOR (M1 half-wired: US2+US5 folded into M1; last-pane refusal moved to T026), 4 MINOR; fixed; analyze HIGH C1/U1 fixed (T006a, T002) |
| Tasks | 2 | e3ce4bb44072efb9b7df77aca2989996be758acd:5a7cd75a2dbce27d266c19362ff9763f1b66a99a | CHANGES: 1 MAJOR declined, 2 MINOR fixed prose only; done |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| — | Tasks 2 | F1 MAJOR: T036/T037 in no milestone | Polish changes no code: left to the close unit (milestones.md rule 4) |

## Handover

M1 in progress on branch `claude/project-thread-wysm57` (from origin/main 909f4b23). Not pushed, no PR yet. Part 1 commit f7db6d74; part 2 commit follows it.

Done: T001–T008, T006a. Core `pane_layout.rs`, wire 37, daemon multi-terminal streaming (part 1). Part 2 (client state, T008 + most of T009): `App.grids` keyed by `TerminalRef`; `App.pane_layouts` (per project), `viewed_terminals_sent`, `viewed_dirty`, `pane_synced_displayed`; `App::displayed_terminal()` (active session + its selected process); `crates/micold-client/src/shell/panes.rs` with `sync` (called from `update`: layout follows the selection via `show_or_focus`, prunes gone terminals, sends `SetViewedTerminals` only when more than one pane or when leaving that), `split_pane`, `focus_pane`/`follow_focus` (selection follows the focused pane), `layout`. `on_terminal_bytes` names `process: Some(..)` only with several panes. Tests: `main_tests.rs` `mod panes` (6 pass). Whole workspace clippy `-D warnings` clean; `micold-client` tests pass. `split_pane`/`focus_pane` carry `#[allow(dead_code)]` until T013/T016 wire them: remove it then. Disk filled once: `rm -rf target-shared` was done (approved).

Open in T009: `PaneMsg` (Split, Show, FocusPane, Resized) in `features/session.rs`, `App.pane_sizes` replacing `last_grid` (`last_grid` kept so far; `send_pane_size`, `on_terminal_resized` still send `process: None` and the focused size only), per-pane selection/scroll (selection and `display_offset` are still single, for the displayed terminal; a `ScrollbackResponse` names no process so it lands on the displayed terminal's grid).

Next: T009 rest, then T010–T020 (SplitView widget in `ui/material/split_view.rs`, pane rendering in `ui/terminal.rs` with a `terminal_pane` per leaf, pane header split buttons + empty-pane picker, focus mark, key routing and one-press focus, per-pane resize and drag coalescing, docs `docs/user-guide/terminal-panes.md` + `docs/SUMMARY.md` link). T018's daemon half is already in `pane_terminals.rs`. Then verify.md: scoped gate + Review A/B + visual pass + full gate + PR file at the scratchpad path in the prompt.

Open findings: none.

## Open escalation

None.

## Follow-ups not done

None yet.
