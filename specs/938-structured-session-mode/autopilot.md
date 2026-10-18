# Autopilot ledger — #938 structured-session-mode

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 938 — issue #938 "Structured session mode: run claude with stream-json next to the PTY mode"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #938
- **Worktree branch**: claude/project-thread-cl0lw3
- **Started**: 2026-10-18
- **Phase**: clarify
- **Next step**: clarify unit — apply the user's answers to the two open questions (Open escalation), then run the next clarify round

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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | e63810803ba37fa21cbd87694781aa84aa9312bd:8ee4531564743d6032729cd056a73f655abfb1a0 | CHANGES: 3 MAJOR, 3 MINOR (all fixed) |
| Review spec | 2 | fd5463849846bf112086f77d7913c42278133170:8ee4531564743d6032729cd056a73f655abfb1a0 | CLEAN (1 MINOR, fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, category 1 (product/scope decision the repo does not settle):

1. How does the user choose the structured kind, and what does a structured session's pane show before #939?
   - (Recommended) A: a per-session choice in the new-session flow (beside the existing AI CLI chevron), Claude Code only; the pane shows a plain read-only line-per-event log until #939. Evidence: docs/user-guide/worktrees-and-sessions.md "Starting, switching, and closing sessions" already picks the AI CLI per session from a chevron; issue #938 says "Selectable per session".
   - B: no UI control yet; only an experimental setting, same plain log.
   - C: daemon and wire only; no client control or display (tests and the agent tool server create structured sessions).
2. Does a structured session's event history survive a daemon restart?
   - (Recommended) A: daemon memory only; after a restart the resumed session shows events from the resume on; earlier history is #939's (it reads the CLI's own transcript on disk). Evidence: keeps #938 to the foundation; the CLI's transcript is already a local record (Principle IV).
   - B: persist each event history to disk under the existing "Save terminal history" setting and replay it after a restart. Evidence: docs/user-guide/settings.md#terminal saves terminal scrollback across service restarts (default on), so B is parity.

## Follow-ups not done

