# Autopilot ledger — #1728 orchestrator-config-section

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: feat: `[orchestrator]` config section and `config check` rules (#1728); labels: flow:feature, effort:low
- **Kind**: feature
- **Effort**: low
- **Issue**: #1728
- **Worktree branch**: feat/1728_feat-orchestrator-config-section-and-config-check
- **Started**: 2026-11-05
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1745 | Design | merged | eaaca02fd7fa5893f63520699c9ccd12f28e494d |
| #1752 | M1 | merged | fa3664a38bde19508f8ad4215fc28ccc8a6d3a7f |
| #1759 | M2 | merged | c63bfc7c625e9cac5e814565ff80e2d20296bb2c |
| #1826 | M3 | merged | 9bc969097f431c906b2dcf3fc2c03de4316d8137 |
| (this PR) | Close | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T007, T003a | full | `[orchestrator]` table parses, rules O1–O5/V4 with positions, user guide | #1752 | merged |
| M2 | T009–T013 | full | `--files` checks, no-leak tests, reload keeps last config | #1759 | merged |
| M3 | T014–T020 | full | refuse url credentials/fragment, close test gaps from tdd/verification.md (close unit found unbuilt behaviour) | #1826 | merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 71e2ac6249f9a3555746e2f8d1ccc0246b4c2dc3:11174021f72bbe06aa2470e521894b2accad8a6e | CHANGES: 4 MAJOR, all fixed (spec prose; fixed, prose only) |
| Spec | 2 (full; snapshot stale after rebase) | 76ef2a27113bda2c7a5166c38d5f2f7cc42ea2ee:656b78ae3b753338ce5149e495d91ced3919b6f7 | CHANGES: 1 MAJOR (anchor wording, aligned to the table; fixed, prose only), 2 MINOR fixed. Numbering FR-010/FR-011 reordered, plan ref relabelled |

| Plan | 1 | 573f45826c468a2ca44df1bab5ec62c891f3fc8b:26fcb3b039574a30aac4878ca1a31ac18a4a7c6e | CLEAN; 3 MINOR fixed (prose only: header anchor D7, Windows wording, config_watch.rs) |
| Tasks | 1 | 4e2263b78b027e73b33b7aac1b5c4db3c8625e09:78ed34c3419f7495841225d1ca695c4749d5e46c | CHANGES: 1 MAJOR (T008 test order), 3 MINOR; all fixed (T003a moved before code, view-state test, Verify, T012 note) |
| Tasks | 2 | 04bb849807d5e2d3fc5e5f3ee44a5c3faa90e833:78ed34c3419f7495841225d1ca695c4749d5e46c | CLEAN; 1 MINOR fixed |
| Code (B, conformance) | 1 | da433e0dc2af8da78132ac6135c7a4d3ac6744d6:3ef4115e85dfffe4d64ebe53a3d39dd4058d0900 | CLEAN |
| Code (A, high) M3 | 1 | c17aefc80d5d7b52b4d655fb88689f5681c69eda:6560c1e1b27779bf08ad4b5521bf8e66d24c34b5 | 2 MINOR test assertions fixed (no `@` in message, duplicate message); port validation declined (pre-existing follow-up, out of M3 scope); O3+O4 double report declined (follow-up) |
| Code (B, sonnet) M3 | 1 | same | CLEAN; 1 MINOR fixed |
| (unused) | - | c17aefc80d5d7b52b4d655fb88689f5681c69eda:6560c1e1b27779bf08ad4b5521bf8e66d24c34b5 | - |
| Code (A, high) | 1 | 4b87210200a365755cfba4d6984e645872b543eb:eaaca02fd7fa5893f63520699c9ccd12f28e494d | CLEAN; 3 MINOR (port not validated, userinfo echoed (fixed in M3), O3+O4 both fire) left as follow-ups |

## Declined review findings

T016 waiver (M3): no `tdd/test-list.md` or `tdd/cycle-log.md` was written for M1/M2 and none is invented now. M3 itself was test-first: the new `o2_a_url_with_credentials_a_fragment_or_no_host…` test failed on the old `check_https_url` (red recorded in the M3 run, 109 passed / 1 failed) before the fix; the O4 and `config_check` tests pin existing behaviour and passed at once. Mutants were not re-run with cargo-mutants.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. M1 #1752 and M2 #1759 merged (M2 CI had one flake, `structured_permission::two_sessions_waiting_are_marked_separately`, green on rerun; the local-gate failure of `ai_cli_config a_configured_model_is_claude_codes_default_launch_model` was a HOME-dependent test outside this diff). Close unit ran converge (no unbuilt tasks) and tdd-verify (FAIL: url credentials/fragment unbuilt, O4 mutant survived, no TDD evidence recorded); cut M3 from `tdd/verification.md`. T016 (no test-list/cycle-log for M1/M2): record a waiver in this ledger rather than fabricate evidence.

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

- M1 review A MINORs: `url` port not parsed; `user:pass@` userinfo would be echoed in the view and O2 message; O3 and O4 both fire for a lone cert/key plus token.
- Gate: `refused_mutations_are_logged` and `structured_permission::two_sessions_waiting_are_marked_separately` each failed once under load, unrelated to this diff (the latter passes alone and in the second full gate).
