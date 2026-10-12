# Autopilot ledger — #489 ssh-remote-daemon

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Run the session daemon on a remote host over SSH (GitHub issue #489); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #489
- **Worktree branch**: feat/489_run-the-session-daemon-on-a-remote-host-over-ssh
- **Started**: 2026-10-12
- **Phase**: tasks (reviewed, clean; design PR open)
- **Next step**: milestone M1 after the design PR merges

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T027 | full | Remote over SSH daemon connects (add, start, Connected) | — | pending |
| M2 | T028–T038 | full | Remote projects, worktrees and sessions | — | pending |
| M3 | T039–T050 | full | Reconnect after a network drop | — | pending |
| M4 | T051–T055 | full | Local work unaffected, concurrent clients | — | pending |
| M5 | T056–T065 | full | Host picker, failure reasons, daemon management | — | pending |
| M6 | T066–T072 | full | Version mismatch explained | — | pending |
| M7 | T073–T081 | full | Replace a mismatched daemon with no sessions | — | pending |
| M8 | T082–T084 | full | Remote daemon artefact for macOS and Windows | — | pending |

## Decisions

- Clarify Q1: daemon delivered by the client copying its own matching build over SSH to a per-user dir _(decided by user)_.
- Clarify Q2: mismatched remote daemon is explained only; replacement offered only with no running sessions _(decided by user)_.
- Tasks: M1 kept whole at 27 tasks (Setup and Foundational are not observable alone; P1 story must be in M1); US1 split scenario 1 / 2-4 (M1/M2); US5 split M6/M7; Polish M8 changes build config, so it is a milestone.
- Tasks: speckit-analyze round 1 found 1 HIGH (shared picker widget) and 5 MEDIUM; all fixed in tasks.md, no plan change.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | ba77a079ad4169573eddc31e57ce119656d81c5f:13165b3a9e4938a42528aae5df288475aa63ddae | CHANGES: 4 MAJOR (reconnecting state, timeout value, backoff cap, session state while disconnected), all fixed |
| Spec | 2 | 0c21e1c00af41296ea2daf2c724c93e197d42c98:13165b3a9e4938a42528aae5df288475aa63ddae | CHANGES: 1 MAJOR (canonicalize_best_effort is a lexical walk, not verbatim: now a required change), 3 MINOR; fixed |
| Plan | 3 | dfcf4d63a3a9a5e42df4dfb44d48c741b07de2e9:e022a929bab01046ada386829ee3f6fa1e62e347 | CLEAN (1 MINOR fixed) |
| Plan | 1 | 9a5ff7ae39fe35fcffdb2f2c26b8dcbe452da2be:c33ad03cfb1fec6ccb1c1856d766226a886e57b6 | CHANGES: 3 MAJOR (local Unreachable vs FR-008; R5 audit list incomplete; R3 cross-OS matrix/glibc), 3 MINOR; all fixed |
| Plan | 2 | 8194452ff2f9a451e06c04598eebc2d0f00299a9:2c9c6abed776f4cdbdf710d33f6c9bea62dd5df1 | CLEAN (round 3 followed) |
| Tasks | 1 | f72a58cd6bb526d69c196ac5286ceb5e01e46526:7b87ef147273f34743f12f80b97367aea13812bc | CLEAN (3 MINOR fixed, prose/task wording; checklist ticked) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Issue #687 duplicates this feature's scope; close it with the final PR.
- Spec 491's tree shows SSH as out of scope; no change needed.
- Plan R3: macOS/Windows clients need a bundled `linux-x86_64` daemon to reach Linux hosts (adds daemon size to those packages); tasks sequence it last and separable. Owner to confirm packaging size.
