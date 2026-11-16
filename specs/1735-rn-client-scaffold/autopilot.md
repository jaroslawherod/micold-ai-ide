# Autopilot ledger — #1735 rn-client-scaffold

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1735 — issue #1735 "feat: React Native (Expo) web client scaffold, Material 3 theme from the tokens, sign-in, sidebar and overview"; build against a mocked orchestrator API behind a TypeScript interface; i18n from the start; use #1950 tokens; plan 3-4 milestones. labels: effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1735
- **Worktree branch**: claude/project-thread-hzt36f
- **Started**: 2026-11-16
- **Phase**: spec
- **Next step**: spec review round 1 NOT run (this unit had no Agent tool): dispatch autopilot-reviewer (opus, rubrics/spec.md) on spec.md first, then clarify unit: answer the 2 `[NEEDS CLARIFICATION]` markers in spec.md (locales; mock in deployed bundle)

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

| Review | Round | Scope | Head | Snapshot | Verdict |
|---|---|---|---|---|---|
| Spec | 1 | spec.md | n/a | e4eb498403faa61c433088be59dc8d505a145b5f:f6aa5b80601f9ce5ef93caddda0c35d1c1c1435e | not run: no Agent tool in this unit |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- #1732 and #1733 not built: the mock must be swapped for the real client and generated types (constitution V) in a follow-up once they land.
