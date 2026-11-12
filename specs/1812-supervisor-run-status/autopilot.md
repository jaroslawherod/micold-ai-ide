# Autopilot ledger — #1812 supervisor-run-status

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: autopilot supervisor M1: machine-readable run status and non-blocking escalation, issue #1812 (see issue body); labels: enhancement, in-progress, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1812
- **Worktree branch**: feat/1812_autopilot-supervisor-m1-machine-readable-run
- **Started**: 2026-11-10
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1819 | design (spec, plan, tasks) | merged | 5e90aef10191fe1b8f8801e63355c8a244a5eef0 |
| #1823 | M1 | merged | acc28c34cd44b5d3eafac49cec605878b4007433 |
| #1830 | M2 | merged | 8753adb2d4b25d900aa2b6558d07843712a158af |
| #1850 | close | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014, T019 | full | run-status.sh and its per-state tests (US1 + US3) | #1823 | merged |
| M2 | T015–T018, T020–T024 | full | supervised escalation: supervised.sh, answer.sh, skill prose (US2) | #1830 | merged |

## Decisions

US3 (tests only) is folded into M1 (rule 7, tests travel with code); US2 is M2 (scripts + skill prose: not light, adds a ledger-writing script). No Polish milestone: no code left.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 1 |  |  | 2d70083d087364f8dc5908be94adf0ee0c21b4ef:e82cc8df6bddd76f87e2d0786098f9d4ceaf8b28 | CHANGES: 3 MAJOR |
| Spec | 2 |  |  | a6d59dc504310543054184cb26c5aa527d345d62:e82cc8df6bddd76f87e2d0786098f9d4ceaf8b28 | CLEAN (1 MINOR fixed) |
| Plan | 1 |  |  | cdc0fc6e30bf489c61a4a8f414e6dc44d4b1f749:dc817a78f2bc0a282455f5c21afbe7eb77fb7056 | CHANGES: 4 MAJOR, 3 MINOR (all fixed) |
| Plan | 2 |  |  | defe853eceacab7ba1241c8b27afdd856c54de92:dc817a78f2bc0a282455f5c21afbe7eb77fb7056 | CLEAN (1 MINOR fixed, prose only) |
| Tasks | 1 |  |  | ed4856e13049fb820ec0fa7c3c53ea472bc12c34:ebe5384e95fabe60e32205366e58241d7ad4414c | CLEAN (3 MINOR fixed, prose only; analyze HIGH C1-C3 fixed first) |
| Review A M1 | 1 |  |  | a1b19852e1c9e0bb9b71caf4e9d3b694cdae72bc:5e90aef10191fe1b8f8801e63355c8a244a5eef0 | CHANGES: 1 MAJOR (fence), 2 MINOR (F2 fixed, F3 declined); fixes prose-free, covered by new tests |
| Review B M1 | 1 |  |  | 59b1a52a21d668ef829e1b47d88795cf30ca715a:5e90aef10191fe1b8f8801e63355c8a244a5eef0 | CLEAN (1 MINOR fixed, ledger only) |
| Review A M2 | 1 | 1830 | 49b48e92353d057accb2a2641f2c1e58a1411b09 | 5f2bf79f69c45300ea1fc1034b8ed571fdf77627:49b48e92353d057accb2a2641f2c1e58a1411b09 | CLEAN (3 MINOR left: answer.sh double message, unknown batch number, no lock; re-review on current head) |
| Review A M2 | 2 | 1830 | d9cc520954bec73e975512f2f920ec1043cbd53d | f1a0b9ca0a981182c4f482814f7dd4034314549a:d9cc520954bec73e975512f2f920ec1043cbd53d | CLEAN (merge of origin/main; conflict in autopilot.test.sh kept both sides) |
| Review close | 1 | close | d2ecabb4e462968cb2a6a75e82f63e85eeba7f7d | 899d90a1216c0c0f21d68de2a166abb3b6eed3fa:8753adb2d4b25d900aa2b6558d07843712a158af | CLEAN (2 MINOR: ledger fixed, live-run note declined) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | F3: several all-done ledgers take the first | A finished run with two done ledgers is not an error; exit 3 stays for open ledgers (research D4) |


## Handover

None. M2 merged; close PR is the last.

## Open escalation

None.

## Follow-ups not done

Full gate not run locally in M2 (queued behind another build); only scripts, tests and skill prose changed, `mise run test-scripts` green, CI validates. Review A: 3 MINOR (2 fixed; blank-answer/usage exit code left). Review B: clean (drift-M2.md added).

None yet.
