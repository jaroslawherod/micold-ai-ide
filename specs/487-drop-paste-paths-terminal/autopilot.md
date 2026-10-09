# Autopilot ledger — #487 drop-paste-paths-terminal

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #487 "Drop files or paste images into a session terminal to insert their paths" (proposal and acceptance criteria per the issue); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #487
- **Worktree branch**: claude/project-thread-wysm57
- **Started**: 2026-10-09
- **Phase**: milestone M1
- **Next step**: M1 gate, reviews A and B, push (PR body in scratchpad pr-487-m1.md)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #661 | Previous run (#484) | merged | a42cf25517a42ad7a17a25a84db513fea48a4565 |
| #662 | Design (spec, plan, tasks) | merged | ab2f9967b89658219500097833054c14634698c3 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | drop files insert quoted paths | | pending |
| M2 | T018–T024 | full | paste image into AI session | | pending |
| M3 | T025–T029 | full | sandbox container paths, refusals | | pending |
| M4 | T030–T034 | full | cleanup on delete and startup | | pending |

## Decisions

- M1: the planned `CursorMoved` subscription is banned by `tests/idle_subscriptions.rs`; the pointer lives in `SplitView`'s widget state and the widget publishes `PaneMsg::FileDropped(pane, path)`; `shell::drops` coalesces with a 40 ms settle (research.md, M1 spike findings). No pane rectangles are published to app state, so T015 became `PaneLayout::pane_at` + the widget hook.
- M1: visual pass skipped: M1 adds no visible element (errors use the existing snackbar) and a file drag cannot be synthesised on Xvfb.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 8577d9164166df52b5472b80d7aab8e487ee78ee:a42cf25517a42ad7a17a25a84db513fea48a4565 | CHANGES: 3 MAJOR (fixed) |
| Spec | 2 | (fix diff, sonnet) | CLEAN (1 MINOR) |
| Plan | 1 | cf9fa8c2fa31216b4d33a2826532e036bfc60e5a:9a2546f998dc3f8911b8404479a2a297555d6919 | CHANGES: 1 MAJOR (fixed) |
| Plan | 2 | scoped fix diff | CLEAN |
| A M1 (code-review high) | 1 | 8f8d224863015e668a987938c539ac181beb136f:16e690ab28a89a13168bb4d176596569bd609fb8 | CHANGES: 1 MAJOR (fixed: refuse any control character when unbracketed), 2 MINOR (CursorLeft clears pointer fixed; shell-detect limit commented) |
| B M1 (conformance, fresh) | 1 | same | CLEAN (4 MINOR: doc order fixed; shell limit noted; no cycle-log; no widget-level FileDropped test, declined) |

## Declined review findings

- B M1 F2 (widget-level FileDropped test): the pure hit-test, the reducer, the coalescing and the wiring are tested; a headless widget harness for one event path was judged out of proportion. Pointer-staleness during a native drag is an unverified platform risk recorded in research.md.
- B M1 F4: no `tdd/cycle-log.md`: tests were written beside the code; red-first was not recorded and is not invented now.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.

- Tasks review (round 1, snapshot 3a1a5833:788b5d57): CHANGES; F1 (paste_source test task) fixed, F2-F4 MINOR fixed; no further round (tasks added, tests-only).
