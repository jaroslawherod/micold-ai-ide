# Autopilot ledger — #938 structured-session-mode

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 938 — issue #938 "Structured session mode: run claude with stream-json next to the PTY mode"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #938
- **Worktree branch**: claude/project-thread-cl0lw3
- **Started**: 2026-10-18
- **Phase**: tasks
- **Next step**: tasks review round 1 (fresh reviewer, rubrics/tasks.md), then checklists, specs-only gate, design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010 | full | parser types every recorded fixture line; kind persisted | — | planned |
| M2 | T011–T021 | full | daemon runs a structured session and streams events; lifecycle parity | — | planned |
| M3 | T022–T028 | full | first prompt and user messages | — | planned |
| M4 | T029–T030 | full | resume | — | planned |
| M5 | T031–T032 | full | late and multiple viewers | — | planned |
| M6 | T033–T036 | full | terminal-only requests refused | — | planned |
| M7 | T037–T047 | full | chevron entry, row marker, log pane, user guide | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | Permission behaviour of a structured session before #942 | Same permission configuration as a terminal session, no override; approvals the CLI would need are refused and show as events (FR-006a) | agent | crates/micold-core/src/provider.rs ToolServerSupport; spec FR-006 |
| 2 | clarify | How the structured kind is chosen; pane before #939 | Per-session, start-session chevron, Claude only; plain read-only line-per-event log (FR-022, FR-022a) | user (provisional: recommended option) | worktrees-and-sessions.md chevron |
| 3 | clarify | Does event history survive a daemon restart | No: daemon memory only; history after restart starts at the resume; #939 reads the CLI transcript (FR-021a) | user (provisional: recommended option) | — |
| 4 | clarify | Where the structured choice is offered | "Claude Code (structured)" entry in the start-session chevron, chevron shown even with one CLI; other entry points create terminal sessions | agent | worktrees-and-sessions.md#choosing-which-ai-cli-a-session-runs |
| 5 | tasks | Milestone cut | M1 = Setup + Foundational + US2 parser (P1); US1 split along scenarios 1–3 / 4–5 (M2, M3), as US1 with the foundation exceeds ~10 tasks; US5 lifecycle tests moved into M2 where the Proc enum is built; added US6 (P3) to spec for FR-022/022a/023 acceptance scenarios; M2 at 11 and M7 at 11 tasks kept whole (no smaller observable split) | agent | speckit-analyze C1, milestones.md rules 1–3 |
| 6 | tasks | Plan fixes from speckit-analyze | `start_new_of_kind`/`restored_of_kind` delegation instead of changing 300 callers (I1); PROTOCOL_VERSION bumped per wire-changing milestone (M2, M3, M6), since each merges separately; `OversizedOnWire` variant added to data-model; FR-014 names daemon notes | agent | tests/schema_hash.rs; analyze I1/F4 |
| 7 | tasks | Fixture capture needs a logged-in claude (R11) | M1 unit escalates if none; claude 2.1.302 is installed in the container | agent | analyze F6 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | e63810803ba37fa21cbd87694781aa84aa9312bd:8ee4531564743d6032729cd056a73f655abfb1a0 | CHANGES: 3 MAJOR, 3 MINOR (all fixed) |
| Review spec | 2 | fd5463849846bf112086f77d7913c42278133170:8ee4531564743d6032729cd056a73f655abfb1a0 | CLEAN (1 MINOR, fixed) |
| Review plan | 1 | 45b0fc650d313338200dd10363a89e8a9d0c79d7:62098e2ac5367f62ce4e90774162b79389a152c5 | CHANGES: 2 MAJOR, 2 MINOR (all fixed) |
| Review plan | 2 | 2e543df9ccb26d8f478193d8cdf4e3ebcbb65767:5233609a4f479ef50bc8ee7f209f6969d2f950a2 | CLEAN (1 MINOR, fixed, prose only) |
| Review tasks | 1 | 04c1c7e294dde5831e98cdfcec29ca4f3d9203f8:5eed6c26bf89d047cf687661f1602dd39770ae1e | CHANGES: 2 MAJOR (T042 client call sites; stale checklist — fixed, spec gained a plain-language summary), 2 MINOR (F3 fixed; F4 already noted in T029) |
| Review tasks | 2 | f741a87ee478cc18590ea2227bdf989249aa490d:96c0440120fd62354a731049322b3adec98be5e1 | pending |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| — | Review tasks 1 | F4 T029 partly green before T030 | already documented in T029; only daemon-restart cases are red-first |

## Handover

- Done: tasks.md with 48 tasks and 7 milestones (M1–M7, all `full`); speckit-analyze run and its
  findings fixed (decisions 5–7); tasks review round 1 fixed and committed; checklist closed (all
  items ticked, spec gained an "In short" section); specs-only gate
  (`scripts/check-criteria-observables.sh`) passed before round 1 fixes.
- In flight: tasks review round 2 (sonnet) runs detached; its output goes to
  `/tmp/claude-0/-home-user-micold-ai-ide/544d91ab-a9c0-5abb-b5f7-de6d31a04477/scratchpad/review-tasks-r2.log`
  (ends with `REVIEW_EXIT=`). Wait on it with `scripts/autopilot/hold.sh <log>`.
- Next: act on round 2 (round limit 3; a fix to the "non-technical" checklist item is prose only),
  record its verdict in *Review rounds*, re-run the specs-only gate, push the branch, write
  `$SCRATCHPAD/pr-title.md` (`#938: Structured session mode: run claude with stream-json next to
  the PTY mode — design`) and `$SCRATCHPAD/pr-body.md` (ends `Refs #938`), return `PR: to-open`
  (cloud.md: no `gh`).
- No PR opened yet.

## Open escalation

None.

## Follow-ups not done

