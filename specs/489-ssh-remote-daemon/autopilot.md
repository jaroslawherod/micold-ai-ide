# Autopilot ledger — #489 ssh-remote-daemon

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Run the session daemon on a remote host over SSH (GitHub issue #489); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #489
- **Worktree branch**: feat/489_run-the-session-daemon-on-a-remote-host-over-ssh
- **Started**: 2026-10-12
- **Phase**: spec (done); next clarify
- **Next step**: (clarify round 1 awaiting user answers, see Open escalation) Run the clarify unit on specs/489-ssh-remote-daemon/spec.md: two [NEEDS CLARIFICATION] markers (daemon delivery to host; replacing a mismatched daemon with running sessions)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | ba77a079ad4169573eddc31e57ce119656d81c5f:13165b3a9e4938a42528aae5df288475aa63ddae | CHANGES: 4 MAJOR (reconnecting state, timeout value, backoff cap, session state while disconnected), all fixed |
| Spec | 2 | 0c21e1c00af41296ea2daf2c724c93e197d42c98:13165b3a9e4938a42528aae5df288475aa63ddae | pending |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1 (category 5, product decision; repo does not settle either, checked spec 491, constitution, docs):

- Q1: How does the daemon binary reach the remote host? Options: (a) client copies its own matching build over SSH to a per-user dir (Recommended: no network on host, always version-matched, needs no manual step per SC-001); (b) host downloads a release; (c) user installs it.
- Q2: May the client replace a version-mismatched remote daemon that has running sessions? Options: (a) explain only, user decides; offer replace only when it has no running sessions (Recommended: protects running sessions, FR-006); (b) client may replace after explicit confirmation naming the sessions that end; (c) always replace automatically.


## Follow-ups not done

- Issue #687 duplicates this feature's scope; close it with the final PR.
- Spec 491's tree shows SSH as out of scope; no change needed.
