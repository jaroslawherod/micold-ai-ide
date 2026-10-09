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

M1 in progress, branch `claude/project-thread-wysm57` restarted from origin/main (909f4b23). Not pushed, no PR yet.

Done (committed): T001–T007, T006a. `crates/micold-core/src/pane_layout.rs` (full M1 API: `PaneLayout::{single,split,show,show_or_focus,focus,prune,place,rects,panes,terminals,focused_terminal,pane_showing}`, `Refusal::reason`, consts `MAX_PANES`, `MIN_PANE_COLS/ROWS`; `split(pane, axis, pane_size, min, candidate)` takes pixel sizes). `TerminalRef` lives in `protocol/messages.rs` and is re-exported from `pane_layout`. Wire 36 → 37 landed whole (GridFrame/SessionInput/SessionResize `process`, `SetViewedTerminals`, `SetPaneLayout`, `ProjectSnapshot.pane_layout`; the daemon ignores `SetPaneLayout` until T032). Daemon: `DaemonState::{session_input_to,resize_terminal,terminal,attached_process}`, `server.rs` `sync_terminal_streams` + `viewed_terminals`/`terminal_streams`; tests in `crates/micold-daemon/tests/pane_terminals.rs` (5 pass). Workspace compiles (`cargo check --workspace --all-targets`); `micold-core` tests pass except the root-only permission tests.

Next: T008 (client tests) + T009 (client state: `App.pane_layouts`, `App.grids` keyed by `TerminalRef`, `App.pane_sizes`; every client `SessionInput`/`SessionResize` currently passes `process: None` and the client does not yet send `SetViewedTerminals`), then T010–T020 (SplitView widget, pane rendering, focus mark, per-pane resize, docs `docs/user-guide/terminal-panes.md` + SUMMARY link). T018's daemon half (two panes of one session hold distinct PTY sizes) is already in `pane_terminals.rs`; its client half is open. Then verify.md: scoped gate + Review A/B + visual pass + full gate + PR file at the scratchpad path in the prompt.

Open findings: none. Tip: `scratchpad/fix.py` patches missing `process`/`pane_layout` struct-literal fields from compile errors.

## Open escalation

None.

## Follow-ups not done

None yet.
