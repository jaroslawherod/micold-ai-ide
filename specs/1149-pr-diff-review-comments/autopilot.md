# Autopilot ledger — #1149 pr-diff-review-comments

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1149 "Review comments on a pull request's diff, sent to the agent session to apply and push to the PR" (verbatim in spec.md Input); labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #1149
- **Worktree branch**: feat/1149_review-comments-on-a-pull-request-s-diff-sent-to
- **Started**: 2026-10-20
- **Phase**: tasks
- **Next step**: review tasks, gate, open design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T011 | full | Write, list, tidy PR comments on a PR's diff (whole wire delta, Send for PR not yet offered) | | pending |
| M2 | T012–T022 | full | Send review on a PR: build_pr batch, branch/commit reads, daemon delivery | | pending |
| M3 | T023–T027 | full | Anchoring on hunks, head-follow, outdated | | pending |

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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
