# Autopilot ledger — #489 ssh-remote-daemon

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Run the session daemon on a remote host over SSH (GitHub issue #489); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #489
- **Worktree branch**: feat/489_run-the-session-daemon-on-a-remote-host-over-ssh
- **Started**: 2026-10-12
- **Phase**: plan (written; review round 1 pending)
- **Next step**: Continue plan unit from Handover: review round 1, fixes, then tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

- Clarify Q1: daemon delivered by the client copying its own matching build over SSH to a per-user dir _(decided by user)_.
- Clarify Q2: mismatched remote daemon is explained only; replacement offered only with no running sessions _(decided by user)_.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | ba77a079ad4169573eddc31e57ce119656d81c5f:13165b3a9e4938a42528aae5df288475aa63ddae | CHANGES: 4 MAJOR (reconnecting state, timeout value, backoff cap, session state while disconnected), all fixed |
| Spec | 2 | 0c21e1c00af41296ea2daf2c724c93e197d42c98:13165b3a9e4938a42528aae5df288475aa63ddae | pending |
| Plan | 1 | 9a5ff7ae39fe35fcffdb2f2c26b8dcbe452da2be:c33ad03cfb1fec6ccb1c1856d766226a886e57b6 | pending (fresh dispatch; earlier reviewer lost) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Plan unit 489, written at over 150k context.
- Done: plan.md, research.md (R1-R13), data-model.md, contracts/ssh-transport.md, contracts/remote-daemon.md,
  quickstart.md written and committed (not pushed; the branch was rebased onto origin/main by branch-start.sh, so a push needs `--force-with-lease`).
- Review round 1 returned CHANGES after the handover (see Review rounds); fix F1-F6, do not re-dispatch round 1. If its
  notification never reaches the orchestrator, dispatch a fresh round 1 (full, session model) with rubrics/plan.md; it does not count against the round limit twice.
- Next: act on the review (fix BLOCKER/MAJOR, re-review scoped on sonnet), then set Next step to `tasks unit`. Open no PR.
- Watch in review: R3 cross-OS daemon artefact (macOS/Windows clients), R7 local-runtime Unreachable semantics, R5 remote project identity audit.

## Open escalation

None.

## Follow-ups not done

- Issue #687 duplicates this feature's scope; close it with the final PR.
- Spec 491's tree shows SSH as out of scope; no change needed.
- Plan R3: macOS/Windows clients need a bundled `linux-x86_64` daemon to reach Linux hosts (adds daemon size to those packages); tasks sequence it last and separable. Owner to confirm packaging size.
