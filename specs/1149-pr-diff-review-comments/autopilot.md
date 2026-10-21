# Autopilot ledger — #1149 pr-diff-review-comments

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1149 "Review comments on a pull request's diff, sent to the agent session to apply and push to the PR" (verbatim in spec.md Input); labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #1149
- **Worktree branch**: feat/1149_review-comments-on-a-pull-request-s-diff-sent-to
- **Started**: 2026-10-20
- **Phase**: implement (M3)
- **Next step**: orchestrator waits on CI, merges, then close

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1153 | Design (spec, clarify, plan, tasks) | merged | 0ebf14ba355194296f2d6d1b0c51597140a601d3 |
| #1158 | M1 (T001–T011) | merged | d9c2e378c1ef6e6a8f142a5369dd16c856e9324c |
| #1172 | M2 (T012–T022) | merged | bc162e1d6d6812afaf38c3987257bf3fe17e99e8 |
| #1184 | M3 (T023–T027) | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T011 | full | Write, list, tidy PR comments on a PR's diff (whole wire delta, Send for PR not yet offered) | #1158 | merged |
| M2 | T012–T022 | full | Send review on a PR: build_pr batch, branch/commit reads, daemon delivery | #1172 | merged |
| M3 | T023–T027 | full | Anchoring on hunks, head-follow, outdated | #1184 | PR #1184 open |

## Decisions

- Clarify: wrong branch on Send review delivers with a switch-branch preamble; merged/closed PR allows Send review and states the state in the batch (decided by user).

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | bfa4381fe14264233731c90e5798d8826424308b:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | CLEAN (3 MINOR, fixed, prose only) |
| Plan | 1 | 9da7410d0fbbfa3b1efa48b7b2efc964b3c3010b:dc70782b6bf0193f8b7575e71e273abb07ee59f6 | CHANGES (2 MAJOR, 3 MINOR, all fixed) |
| Plan | 2 | c3f8e55734895dfd9d4f21aa89c678c63359b917:dc70782b6bf0193f8b7575e71e273abb07ee59f6 | CLEAN (2 MINOR, fixed, prose only) |
| Tasks | 1 | 2e2a21d914c701e16939ae8cc0f2572fcee721b1:18eb11b47a1f2d64bcfddf0c3eb39eeee17c4324 | CHANGES (2 MAJOR, 2 MINOR, all fixed: test-first order of T020/T021, W21 test into T006) |
| Tasks | 2 | diff HEAD~2 | CLEAN (2 MINOR, no action) |
| M1 diff A (high) | 1 | 2e3a3f4c226dc859315e8577b0fcaf465ad7ef51:7e012c8f1ccd04b1474bd10452a0c78b3493a831 | CLEAN (3 MINOR; F2 fixed) |
| M2 diff A (high) | 1 | 1d57567e3f7e078839c0ccea65cb6b2726802d4e:d9c2e378c1ef6e6a8f142a5369dd16c856e9324c | CHANGES, fixed (7 findings: 2 fixed, 5 declined) |
| M2 diff B (conformance) | 1 | 9aaa2a907f335506a49db36354855054ffc941c6:a673841f4d04a9ee5f0e2f92821a8f1c868ffd6d | CHANGES (1 MAJOR: FR-007 head commit not in preamble; fixed by naming the current head commit in P-1, contract, tests, user guide; 1 MINOR no action) |
| M3 diff A (high) | 1 | ca8447908d38f47484e23e54fde25a0bf0373885:562c909ea9c6427a694b90749b3bbc0ea551aeac | CLEAN (2 MINOR, declined) |
| M3 diff B (conformance) | 1 | ca8447908d38f47484e23e54fde25a0bf0373885:562c909ea9c6427a694b90749b3bbc0ea551aeac | CLEAN (2 MINOR; clippy allow on Lines committed) |
| M1 diff B (conformance) | 1 | 1d902dec2d126f965bf0b7be9a7a95485058dc3c:7e012c8f1ccd04b1474bd10452a0c78b3493a831 | CLEAN (3 MINOR) |

## Declined review findings

- Review A F1 (jump from another scope's panel row does nothing): MINOR, kept per brief (jump_target filters by shown scope); selecting that PR is M3/later.
- Review A F3 (toolbar label counts shown scope): per brief. Review B F1 (heading prefixes each row): VirtualRows has one row height. Review B F2 (no tdd/ dir): tests for core/daemon written first; client tests written after the implementation, recorded here.

- M3 Review A F1 (composer pinned after re-read keeps old line numbers) and F2 (large unshown patch keeps prior outdated): MINOR, edge cases; the composer anchors by commit and the large patch is judged once shown. Review B F2: head re-read reuses Effect::ReadPullRequestDiff, no shell change needed.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
