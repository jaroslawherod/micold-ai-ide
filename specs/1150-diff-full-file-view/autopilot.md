# Autopilot ledger — #1150 diff-full-file-view

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1150 "Show the full file and browse the repo's code while reviewing a diff in the Changes view" (full text via gh issue view 1150); labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #1150
- **Worktree branch**: feat/1150_show-the-full-file-while-reviewing-a-diff-in-the
- **Started**: 2026-10-20
- **Phase**: tasks
- **Next step**: tasks reviewed, gate, open design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | Expand hidden lines in a diff (MVP) | | pending |
| M2 | T018–T022 | light | Whole-file toggle | | pending |
| M3 | T023–T031 | full | Pull request head version | | pending |
| M4 | T032–T035 | light | Pick revealed lines | | pending |
| M5 | T036–T046 | full | Browse repository files | | pending |
| M6 | T047–T053 | full | Browse at the PR head | | pending |
| M7 | T054–T062 | full | Follow references from the diff | | pending |

## Decisions

- M1 has 17 tasks (Setup + Foundational + US1): Foundational alone is no milestone, and no acceptance-scenario split exists below the gap marker. Polish (T063-T064) changes no code: left to the close unit.
- analyze C1/C2: browsed-file pick pin moved to M5 (T038); M4 mirrors the existing line-pick handling.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 61e32c1ce6afda347a1774dc7b2c2ebd349b7abd:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | round 1 CHANGES: 2 MAJOR (F1 reference undefined, F2 picking in untouched files); all fixed, 3 MINOR fixed |
| Spec | 2 | ebdf38fc38d5021afe13cc798bd796d24dabf88b:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | CLEAN (1 MINOR fixed, prose only) |
| Plan | 1 | a062beaa985a9a6340ed2e978a69d8ea0fe6978c:6179267cd162fbbfa13ebaa171f85c55810ee0be | CHANGES: 3 MAJOR (PR trailing gap count, hunk-header conventions, test-layer map), 4 MINOR; all fixed |
| Plan | 2 | 3213adb2c03edaa4431e840ab07b5a36c88de031:6179267cd162fbbfa13ebaa171f85c55810ee0be | CLEAN (3 MINOR fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
