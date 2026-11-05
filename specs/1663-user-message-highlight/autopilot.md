# Autopilot ledger — #1663 user-message-highlight

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1663 "Structured view: messages sent by the user are not visually distinct from the agent's output" (tinted surface + left accent for user turns: typed, queued-delivered, slash commands; theme tokens; contrast; selection #1550; visual pass). labels: none (now flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1663
- **Worktree branch**: feat/1663_structured-view-messages-sent-by-the-user-are-not
- **Started**: 2026-11-05
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1695 | design | merged | 8a4e2dd39e9ed9d7acb2f2f4743dc89ffbd0567b |
| #1701 | M1 | merged | 80d691d1580e1af4d771edabe9135b5a08c0a8f9 |
| close | close | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010 | full | Every user turn shows tinted surface + 4dp primary left edge, light and dark; selection unchanged; guide updated | #1701 | merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Tint+accent or right alignment | Tint + left accent, full width | agent-resolved | spec.md#Assumptions |

## Review rounds

- tasks round 1 snapshot: 310509aaeb23c63fae305dd51dbe390d7b0cbdf9:287bae52c9187a35edb3df4479dbf4c789d222d9 (after speckit-analyze fixes: probe, command-output pair, selection colour, fold-state test)
- tasks round 1: CLEAN (2 MINOR: T006 [P] dropped; T001 label left, prose only)

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 6911cba97a4f081d2cad1cc6b9eeb0fef5bc3891:93bc2a7f9a0a8723aa5ec8061cfe36c235c9f6f3 | CLEAN (3 MINOR, fixed prose only) |
| Plan | 1 | 376151e778b78200cd75a9b4d2f7044881ea765f:82db0e58b277c30ba4d849a4598ba04e62761bc1 | CHANGES (1 MAJOR test-observable named in plan; 3 MINOR fixed); fixes are plan-text clarifications, checked by me, no round 2 |
| Plan | 2 | b4a8e147fa5bd3f82da8bfbc4a8fd3944d37484c:1c3512f6cb4caa790413b657e577fb34ccfee0be | CLEAN (2 MINOR; floor lowered to 1.05, prose only) |
| Review A M1 (code-review high) | 1 | 7eb08d8cf5e37cbf8ceb10ce42b301e99161a722:8a4e2dd39e9ed9d7acb2f2f4743dc89ffbd0567b | 8 findings, 7 fixed (showcase entry, edge corner radius, PhantomData, two-turn test, wrap once, selection colour is `primary_container` not alpha, T010 pending), 1 declined (double render in `highlighted`, MINOR test cost); fixes are small and no BLOCKER, no round 2 |
| Close diff review | 1 | 88bf25eb400363ff57e38eef16d7a38c220b3728:b6f3173952ee2bc214f7ca830444cfdd79e54616 | CLEAN (2 MINOR prose, fixed prose only) |
| Review B M1 (conformance) | 1 | | CLEAN (3 MINOR, cycle-log red evidence, optional) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
