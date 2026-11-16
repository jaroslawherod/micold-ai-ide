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
- **Phase**: tasks written (review pending (orchestrator))
- **Next step**: orchestrator runs the fresh tasks review, then this unit continues for speckit-analyze, gate and design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018 | full | themed app, client-test/client-web, CI job, i18n core, spike recorded | - | pending |
| M2 | T019–T031 | full | protocol package, mock, reconnect, sign-in, mock-free prod export | - | pending |
| M3 | T032–T045 | full | sidebar and Overview in stable order, stale marking, user guide | - | pending |
| M4 | T046–T053 | full | Playwright 3 browsers, screenshots, visual pass, bundle hardening, docs | - | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 1 | spec.md | 7180aecdec89025cc8196b00a2f59e84ddce08e2 | e4eb498403faa61c433088be59dc8d505a145b5f:f6aa5b80601f9ce5ef93caddda0c35d1c1c1435e | CLEAN (3 minor F1-F3 applied in clarify) |
| Plan | 1 | plan.md, research.md, data-model.md, contracts/ | 7d38edbc4 | - | CHANGES (F1-F2 major, F3-F4 minor), all applied |
| Plan | 2 | plan.md, research.md, data-model.md, contracts/ | - | - | CLEAN with minors (sonnet re-review; both applied: `.gitattributes` entry, merged duplicate ci.yml entries) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- #1732 and #1733 not built: the mock must be swapped for the real client and generated types (constitution V) in a follow-up once they land.
