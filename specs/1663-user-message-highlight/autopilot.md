# Autopilot ledger — #1663 user-message-highlight

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1663 "Structured view: messages sent by the user are not visually distinct from the agent's output" (tinted surface + left accent for user turns: typed, queued-delivered, slash commands; theme tokens; contrast; selection #1550; visual pass). labels: none (now flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1663
- **Worktree branch**: feat/1663_structured-view-messages-sent-by-the-user-are-not
- **Started**: 2026-11-05
- **Phase**: milestone M1 (implemented, in review)
- **Next step**: review B, visual pass (T010), full gate, PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1695 | design | merged | 8a4e2dd39e9ed9d7acb2f2f4743dc89ffbd0567b |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010 | full | Every user turn shows tinted surface + 4dp primary left edge, light and dark; selection unchanged; guide updated | | pending |

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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Done (M1, T001-T009 ticked, uncommitted-then-committed as `feat(1663)` WIP on the branch, NOT pushed, no PR yet): `UserTurn` primitive + styles, wrap in `entries.rs`, showcase entry, user guide, tests, layout_snapshot regenerated (+16 = SM padding, intended), `tdd/cycle-log.md`, research R1 addendum and R6. Review A round 1 fixed (see Review rounds). Review B round 1: CLEAN (3 MINOR, all about cycle-log red evidence; optional). Scoped gate green except one flaky daemon test (`refused_mutations_are_logged`, global tracing subscriber; passes alone).

Next for the fresh unit:
1. Visual pass (T010) was dispatched to an `autopilot-worker` (agentId ac20e6e68a058e559) and may still be running or finished: check `git status` for `specs/1663-user-message-highlight/visual-pass.md` and `visual-pass/*.png`, and that `crates/micold-client/src/showcase/samples.rs` is NOT modified (the worker was told to restore it; `git checkout -- ` it if not). If no visual-pass.md exists, run the `visual-pass` skill through a new `autopilot-worker` (brief: quickstart Part B; showcase `UserTurn` entry + transcript sample, light and dark; edge corners follow the rounded surface; back-to-back turns; if they touch add `spacing::XS` bottom padding in `UserTurn`). Tick T010.
2. Fix anything real from the visual pass; scoped gate; then FULL `mise run gate` once (detached, hold.sh); update layout_snapshot only if padding changes (`UPDATE_LAYOUT_SNAPSHOT=1 scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot`).
3. Tasks.md Verify note: client tests are modules of one target: `cargo test -p micold-client --test main transcript_entry_view`, and the material filter should be `--test user_turn`.
4. Ledger: Review B row (CLEAN), M1 row status, then PR per pr.md (title `feat(1663): highlight the user's turns in the structured view (#1663)`, body ends `Refs #1663`).

## Open escalation

None.

## Follow-ups not done

None.
