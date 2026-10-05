# Autopilot ledger — #582 attach-provider-worktrees-sessions

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #582 "AI CLI providers should be able to attach existing worktrees and sessions" (scope verbatim from the issue); labels: none (now flow:feature, in-progress)
- **Kind**: feature
- **Effort**: default
- **Issue**: #582
- **Worktree branch**: feat/582-attach-provider-worktrees-sessions
- **Started**: 2026-10-05
- **Phase**: tasks
- **Next step**: design PR open; after merge, milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016 | full | Attach provider worktrees from the app (dialog) | | pending |
| M2 | T017–T026 | full | Discover and resume provider sessions | | pending |
| M3 | T027–T031 | full | `attach_worktree` and `list_resumable_sessions` MCP tools | | pending |
| M4 | T032–T036 | full | Start-up offer banner | | pending |
| M5 | T037–T038 | docs | Polish: quickstart passes, user guide matches | | pending |

## Decisions

- Clarify rounds: round 1 (FR-011 new read-only MCP tool; FR-015 Default refused outright); round 2 via speckit-clarify (attach session = idle entry; resume attaches its worktree first), all agent-resolved; round 3 (speckit-clarify): "No critical ambiguities detected worth formal clarification" (deferred to plan: dismissing the start-up offer; a session already running outside Micold; "empty catalog" is per project).

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| P1 | plan | Dismissing the start-up offer | per project, per run, in memory; "Attach existing..." dialog stays | agent | research R5 |
| P2 | plan | Session already running outside Micold | not detected; documented | agent | research R7 |
| P3 | plan | 026 auto-adoption vs FR-012 | keep 026 adoption; deviation recorded | agent | plan Complexity Tracking |
| P4 | tasks | Spec/plan agreement: FR-012 vs 026 adoption | spec FR-012 and US4 amended: offer-only for this feature, 026 adoption kept; issue intent (offer recovery) unchanged | agent | plan Complexity Tracking row 1 |
| P5 | tasks | Offer trigger | spec FR-012/US4 say "no provenance records" instead of "empty catalog"; serves the issue's lost-data-directory case | agent | plan row 2, data-model OfferState |
| P6 | tasks | Branch-conflict edge case | spec edge case becomes `NotAWorktreeOfProject` (attach checks out nothing, FR-003) | agent | plan row 3, research R1 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | b67119a47b016c5e78ce29eaa83663d3b5cbabac:7cbb6c7625d8bd870d57acad0b63b2ccbd972282 | CHANGES: 3 MAJOR, fixed (F1-F5); no re-review needed (spec-only fixes, markers become clarify questions) |
| Spec review | 2 | 42d64e3443fbe513de491257035ffb981dcb96aa:fc8206f274fa11d82698d9fa046a2789e83ca4f5 | CLEAN (2 MINOR fixed: FR order, scenario wording) |
| Plan review | 1 | 7f1ddbae640e5bea504fc3ae70ce6070d16b30aa:58eb15027e6609de65fbee294aded0d0e0d34da5 | CHANGES: 3 MAJOR (offer trigger vs 026 adoption, FR-007 listing, no agent resume), 2 MINOR; all fixed |
| Plan review | 2 | 55e3ee04bb1fadcb1a06aa062285101515f7e226:58eb15027e6609de65fbee294aded0d0e0d34da5 | CLEAN (3 MINOR fixed) |
| Tasks review | 1 | 417dbb7f488aec137610fe1c8a19f5693ff81e1c:7d9b479fef775a2c9f7046396a274e6b0b4b5df8 | CHANGES: 1 MAJOR ([P] T018 shared file with T017), 2 MINOR; all fixed (speckit-analyze I1,I2,C2,C3,A1 also fixed) |
| Tasks review | 2 | 16f83e2bb3c3fcd8c606a488ebbbcbd213a3f849:7d9b479fef775a2c9f7046396a274e6b0b4b5df8 | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- branch-start.sh printed FETCH-FAILED (broken refs in local repo: "does not point to a valid object"); branch is at the commit it started on, not re-based on a fresh origin/main.
