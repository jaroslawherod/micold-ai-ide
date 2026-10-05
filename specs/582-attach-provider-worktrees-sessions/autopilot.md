# Autopilot ledger — #582 attach-provider-worktrees-sessions

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #582 "AI CLI providers should be able to attach existing worktrees and sessions" (scope verbatim from the issue); labels: none (now flow:feature, in-progress)
- **Kind**: feature
- **Effort**: default
- **Issue**: #582
- **Worktree branch**: feat/582-attach-provider-worktrees-sessions
- **Started**: 2026-10-05
- **Phase**: milestone M4 (PR open)
- **Next step**: wait for CI, merge M4, then M5

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #585 | Design PR: spec, plan, tasks | merged | a3f66693402b1242bd2e20d658b51549fb22fc9e |
| #587 | M1: attach provider worktrees from the app | merged | 502c49691bc45786f85f2a60a7e13417bf08cabc |
| #588 | M2: discover and resume provider sessions | merged | e849b117222256304d49f6e01e0d449a89c96d5c |
| #589 | M3: attach_worktree and list_resumable_sessions MCP tools | merged | e3cef56de6611c1ce080e7bb5a39fd09c0bd079e |
| #590 | M4: start-up offer banner | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016 | full | Attach provider worktrees from the app (dialog) | #587 | merged |
| M2 | T017–T026 | full | Discover and resume provider sessions | #588 | merged |
| M3 | T027–T031 | full | `attach_worktree` and `list_resumable_sessions` MCP tools | #589 | merged |
| M4 | T032–T036 | full | Start-up offer banner | #590 | in review |
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
| M1 Review A | 1 | e490c2c5707d3cb9064b333ff5ce9cbf9b0f8e97:6977a9592833ebd47531e6584f7c6898eda334a4 | CHANGES: 2 MAJOR (unreadable project, persist rollback) + stale-reply and list-keeping findings, all fixed |
| M1 Review A | 2 | 58e65c5f48816255b5016fa76cce028cc5b61317:ad8191d1410cd917a8003ee27bf3d6838cb6a197 | pending |
| M2 Review A | 1 | a2269948470b3c917a2e4a470126c2d7f352832a:2a84d8d859247c499d1f62b671378527c030a36a | CLEAN (3 MINOR: F2 per-item IoFailed and F3 doc fixed; F1 restored_stub left) |
| M2 Review B | 1 | a2269948470b3c917a2e4a470126c2d7f352832a:2a84d8d859247c499d1f62b671378527c030a36a | CLEAN (2 MINOR fixed: T023 deviation logged, unused param) |
| M3 Review A | 1 | 008f809719dfdb601d404858fe5102758a1bcb39:e849b117222256304d49f6e01e0d449a89c96d5c | CLEAN (3 MINOR fixed: refusal-before-resolution test, visible_refs rename, doc wrap) |
| M3 Review B | 1 | 008f809719dfdb601d404858fe5102758a1bcb39:e849b117222256304d49f6e01e0d449a89c96d5c | CLEAN (2 MINOR: doc wrap fixed; invalid_input branch for an included worktree kept, untested) |
| M4 Review A | 1 | 4dc431a9a600f74b5525391611ff3001a671f12a:e3cef56de6611c1ce080e7bb5a39fd09c0bd079e | CLEAN (3 MINOR fixed: vacuous test dropped, all-failed batch keeps banner, unexpected result variant reports failure) |
| M4 Review B | 1 | 2bad84caf7398a5aa3372772f646b1abf2bed1ee:e3cef56de6611c1ce080e7bb5a39fd09c0bd079e | CLEAN (1 MINOR: offer_visible is evaluated live at render, so a late provenance mirror only hides the banner; left) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A (part 2, late) | F5 disconnect with AttachApply pending reported as ApplyFailed | The text already says it may or may not have taken effect and that reconnecting shows the current state; the dialog stays so the user can see the list refresh. Accepted for M1. |
| M1 | A (part 2, late) | Lower confidence: Session targets refused as Unavailable; persist on async task without spawn_blocking; pending_ops.len() send check; unsorted attachable-first; discarded JoinError; backfill write on AttachDiscover | M1 attaches worktrees only (Session targets arrive in M2); the patterns match existing handlers; accepted for M1. |
| M1 | A (round 1 part 3) | F2 test does not prove "one persist" | MINOR; the test asserts the records, which is the behaviour; not worth a counting store. |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- branch-start.sh printed FETCH-FAILED (broken refs in local repo: "does not point to a valid object"); branch is at the commit it started on, not re-based on a fresh origin/main.
- M2 visual pass (visual/results.md): ~150 px gap between worktree and session lists; dialog lingers ~5 s after Resume (maybe lavapipe); light-theme sidebar stayed dimmed after close (maybe stale frame); an unreadable Copilot store gave no note. Dialog heading/intro still say "worktrees"; Resume buttons lack keyboard focus ids; 200-session cap has no "more" marker.
