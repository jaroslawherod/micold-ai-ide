# Autopilot ledger — #582 attach-provider-worktrees-sessions

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #582 "AI CLI providers should be able to attach existing worktrees and sessions" (scope verbatim from the issue); labels: none (now flow:feature, in-progress)
- **Kind**: feature
- **Effort**: default
- **Issue**: #582
- **Worktree branch**: feat/582-attach-provider-worktrees-sessions
- **Started**: 2026-10-05
- **Phase**: milestone M1 (implement)
- **Next step**: M1 implement: T009, T012-T016 (see Handover)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #585 | Design PR: spec, plan, tasks | merged | a3f66693402b1242bd2e20d658b51549fb22fc9e |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016 | full | Attach provider worktrees from the app (dialog) | | in progress |
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

M1 unit 1 stopped at the 150k context cap. Branch `feat/582-attach-provider-worktrees-sessions` is based on the design merge a3f66693 (local `origin/main` ref was stale; `git fetch` fails on broken refs, so branch-start RESET was done by hand with `git switch -C <branch> a3f66693` and `git update-ref refs/remotes/origin/main a3f66693`; verify main with `git ls-remote origin refs/heads/main`). Commit 429b6152 holds the work, not pushed, no PR.

Done and green (tests pass): T001-T008, T010, T011, ticked in tasks.md. `specs/.../tdd/cycle-log.md` records red evidence (no test-list.md exists; test-first task order used instead).
- core: `attach.rs` (types, `attachable_worktrees(worktrees, &ProvenanceView, &[Session])`), 3 protocol messages + `OperationResult::AttachApplied { results: Vec<AttachResult> }`, PROTOCOL_VERSION 27 -> 28 (schema_hash.rs constant updated).
- daemon: `Catalog::attach_worktrees(project, &[Worktree] live, &[String]) -> io::Result<Vec<AttachOutcome>>`; `DaemonState::{attach_discover, attach_apply}` (Session items refused as Unavailable until M2); server arms for AttachDiscover/AttachApply. `crates/micold-daemon/tests/attach_apply.rs` has 9 passing tests.

Next steps:
1. T009 tests then T012 reducer `crates/micold-client/src/features/attach.rs` (render-free; register in features/mod.rs). Client gates to respect: `features_are_render_free`, `feature_registration_cost` (a feature declaring `pub enum Msg` needs `pub fn update(&mut State, Msg) -> Vec<Outcome>` and an isolation test per `feature_write_isolation.rs`), `logical_state_ownership`. Study `features/help.rs` and `tests/features_help.rs` as the small model.
2. T013 `PendingOp::Attach` in `shell/daemon_sync.rs`; T014 sidebar button + `ui/attach_dialog.rs` (shared primitives, builder API, Constitution VIII).
3. T015 docs `docs/user-guide/worktrees-and-sessions.md`; T016 visual-pass (quickstart B2/B3 worktrees, light+dark) saved in `specs/582-.../visual/`, via an autopilot-worker.
4. Then verify.md / review A+B / full gate / PR per tasks files. Check the full `mise run gate` and macOS cfg not needed.
Build tips: another leftover cargo/flock wait wasted ~25 min; if `build-lock.sh` says another build holds the lock with no cargo running, use `MICOLD_NO_BUILD_LOCK=1`. Never run `pgrep -fa` (dumps huge command lines).

## Open escalation

None.

## Follow-ups not done

- branch-start.sh printed FETCH-FAILED (broken refs in local repo: "does not point to a valid object"); branch is at the commit it started on, not re-based on a fresh origin/main.
