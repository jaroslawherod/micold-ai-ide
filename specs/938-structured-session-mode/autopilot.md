# Autopilot ledger — #938 structured-session-mode

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 938 — issue #938 "Structured session mode: run claude with stream-json next to the PTY mode"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #938
- **Worktree branch**: claude/project-thread-cl0lw3
- **Started**: 2026-10-18
- **Phase**: plan
- **Next step**: plan unit — plan review round 2 running

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | Permission behaviour of a structured session before #942 | Same permission configuration as a terminal session, no override; approvals the CLI would need are refused and show as events (FR-006a) | agent | crates/micold-core/src/provider.rs ToolServerSupport; spec FR-006 |
| 2 | clarify | How the structured kind is chosen; pane before #939 | Per-session, start-session chevron, Claude only; plain read-only line-per-event log (FR-022, FR-022a) | user (provisional: recommended option) | worktrees-and-sessions.md chevron |
| 3 | clarify | Does event history survive a daemon restart | No: daemon memory only; history after restart starts at the resume; #939 reads the CLI transcript (FR-021a) | user (provisional: recommended option) | — |
| 4 | clarify | Where the structured choice is offered | "Claude Code (structured)" entry in the start-session chevron, chevron shown even with one CLI; other entry points create terminal sessions | agent | worktrees-and-sessions.md#choosing-which-ai-cli-a-session-runs |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | e63810803ba37fa21cbd87694781aa84aa9312bd:8ee4531564743d6032729cd056a73f655abfb1a0 | CHANGES: 3 MAJOR, 3 MINOR (all fixed) |
| Review spec | 2 | fd5463849846bf112086f77d7913c42278133170:8ee4531564743d6032729cd056a73f655abfb1a0 | CLEAN (1 MINOR, fixed) |
| Review plan | 1 | 45b0fc650d313338200dd10363a89e8a9d0c79d7:62098e2ac5367f62ce4e90774162b79389a152c5 | CHANGES: 2 MAJOR, 2 MINOR (all fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

