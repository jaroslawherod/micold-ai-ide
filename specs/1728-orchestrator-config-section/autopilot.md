# Autopilot ledger — #1728 orchestrator-config-section

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: feat: `[orchestrator]` config section and `config check` rules (#1728); labels: flow:feature, effort:low
- **Kind**: feature
- **Effort**: low
- **Issue**: #1728
- **Worktree branch**: feat/1728_feat-orchestrator-config-section-and-config-check
- **Started**: 2026-11-05
- **Phase**: milestone M1
- **Next step**: M1 PR open; wait for CI and merge, then M2

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1745 | Design | merged | eaaca02fd7fa5893f63520699c9ccd12f28e494d |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T007, T003a | full | `[orchestrator]` table parses, rules O1–O5/V4 with positions, user guide | #… | in review |
| M2 | T009–T013 | full | `--files` checks, no-leak tests, reload keeps last config | #… | pending |

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
| Code (A, high) | 1 | 4b87210200a365755cfba4d6984e645872b543eb:eaaca02fd7fa5893f63520699c9ccd12f28e494d | CLEAN; 3 MINOR (port not validated, userinfo echoed, O3+O4 both fire) left as follow-ups |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

- M1 review A MINORs: `url` port not parsed; `user:pass@` userinfo would be echoed in the view and O2 message; O3 and O4 both fire for a lone cert/key plus token.
- Gate: `refused_mutations_are_logged` failed once in the scoped gate (global tracing subscriber already set), unrelated to this diff.
