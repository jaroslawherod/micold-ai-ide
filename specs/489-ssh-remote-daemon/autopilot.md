# Autopilot ledger — #489 ssh-remote-daemon

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Run the session daemon on a remote host over SSH (GitHub issue #489); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #489
- **Worktree branch**: feat/489_run-the-session-daemon-on-a-remote-host-over-ssh
- **Started**: 2026-10-12
- **Phase**: M3 PR open
- **Next step**: orchestrator waits on CI and merges M3, then M4

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #789 | Design (spec, plan, tasks) | merged | 63838fce6f95ea5195529109ba0e0b1073530135 |
| #845 | M1 | merged | cf40ca917492368b043af1fe321c15715e919b2d |
| #883 | M2 | merged | ce4dbafccc49cfe6ad011d64b556c1ac5c9e9214 |
| #895 | M3 | open | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T027 | full | Remote over SSH daemon connects (add, start, Connected) | #845 | done |
| M2 | T028–T038 | full | Remote projects, worktrees and sessions | #883 | done |
| M3 | T039–T050 | full | Reconnect after a network drop | #895 | in review |
| M4 | T051–T055 | full | Local work unaffected, concurrent clients | — | pending |
| M5 | T056–T065 | full | Host picker, failure reasons, daemon management | — | pending |
| M6 | T066–T072 | full | Version mismatch explained | — | pending |
| M7 | T073–T081 | full | Replace a mismatched daemon with no sessions | — | pending |
| M8 | T082–T084 | full | Remote daemon artefact for macOS and Windows | — | pending |

## Decisions

- Clarify Q1: daemon delivered by the client copying its own matching build over SSH to a per-user dir _(decided by user)_.
- Clarify Q2: mismatched remote daemon is explained only; replacement offered only with no running sessions _(decided by user)_.
- Tasks: M1 kept whole at 27 tasks (Setup and Foundational are not observable alone; P1 story must be in M1); US1 split scenario 1 / 2-4 (M1/M2); US5 split M6/M7; Polish M8 changes build config, so it is a milestone.
- M1 part 4 run authorised (decided by user: run a 4th part of M1).
- M1 part 7 authorised (decided by user via AskUserQuestion): rebase onto origin/main, run only the scoped checks (fmt, clippy, tests), push, open the M1 PR and let CI be the full gate.
- M1 part 6 authorised (decided by user via AskUserQuestion): rerun the gate with MICOLD_NO_BUILD_LOCK=1, then push and open the M1 PR.
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
| Code A | 1 | 575310e1c7993dc68f2fafa530e09dd835035320:4ef2cba18ab0513f2d054751ad72c07e34988385 | CHANGES: 1 MAJOR (bridge hangs on stdin at runtime drop), 3 MINOR (UpdateHostKeys, ServerAlive, comment order); all fixed |
| Conformance B | 1 | 63a975671eae50a8aec5a9d00f1b2f1f30e15cc4:b3098a8a0a49c93ac8ad31fe0dcc292ba48f1fe3 | CHANGES: 1 MAJOR (no red-before-green recorded outside core/ssh: deviation now written in cycle-log, no red run fabricated), 2 MINOR declined |

| Code A M3 | 1 | 8e76f42d25b6f3cb4abdbba133fccaa69d6b52c9:cc041d5055f93d89b924ad57eb1114474c83cf85 | CHANGES: 1 MAJOR (Reconnect during outage ended the attempts; fixed with ActorSpec.resumed), 2 MINOR declined |
| Conformance B M3 | 1 | a6a3a59724167f056348879bfc89da0d364ddddc:009e6393a452cac9a09765bfc662b80b107e19cc | CLEAN (test-ssh and test-core green), 2 MINOR noted |

## Declined review findings

- M2 A F2 (log in through_json fallback): declined; the fallback is documented and a wrap failure is not reachable for serde-derived messages.
- B F2 (assert user@host:port in ssh_real test): declined; the stand-in test drives `provision::connect`, not the view; the rendering is asserted in `daemons_feature.rs` (T020).
- B F3 (copy has no deadline of its own): declined; the copy step already runs with `deadline = None` and progress, and `ServerAliveInterval/CountMax` now bound a dead network.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
- M3 A F2/B F1 (local runtimes show Reconnecting): FR-008 says both; kept for every runtime, spec owner may narrow.
- M3 A F3 (generic `connection lost` reason until the first dial fails): MINOR, the reason updates on the first failed attempt.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- `session_lost_label(.., absent_from_catalog)` (links.rs) has no production caller: the daemon-restarted case needs a catalog comparison on reconnect (M3 shows `disconnected`/`unreachable` via `unavailable_label`). Wire it in a later milestone.
- Issue #687 duplicates this feature's scope; close it with the final PR.
- Spec 491's tree shows SSH as out of scope; no change needed.
- Plan R3: macOS/Windows clients need a bundled `linux-x86_64` daemon to reach Linux hosts (adds daemon size to those packages); tasks sequence it last and separable. Owner to confirm packaging size.
