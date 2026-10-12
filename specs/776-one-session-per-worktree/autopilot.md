# Autopilot ledger — #776 one-session-per-worktree

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 776 — "One session per worktree, chosen at start" (issue #776, verbatim in the issue); labels: enhancement, in-progress, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #776
- **Worktree branch**: feat/776_one-session-per-worktree-chosen-at-start
- **Started**: 2026-10-12
- **Phase**: tasks (done, design PR open)
- **Next step**: milestone M1 after the design PR merges

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #791 | Design (spec, plan, tasks) | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Second session refused by daemon (RPC, create_session, review send); wire fields + v41 | | pending |
| M2 | T014–T016, T021 | full | create_worktree / WorktreeCreate define the session at start | | pending |
| M3 | T017–T020, T022, T036 | full | Dialog CLI/prompt fields; no add action on occupied rows | | pending |
| M4 | T023–T030 | full | Resume replaces with confirmation; attach cap | | pending |
| M5 | T031–T035 | full | Legacy catalogs intact; start refused while another is live | | pending |

## Decisions

- Tasks: optional hooks (docguard review, tdd plan) skipped; test tasks are ordered first by hand. All wire additions and the v41 bump land in M1 so later milestones need no second bump. Foundational+US1 server half is 13 tasks; kept whole (no split gives an observable deliverable earlier). Polish has no code: quickstart results are left to the close unit.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | aa9ff7d5e93d7636fe58e91e03abeb878a2c199b:13165b3a9e4938a42528aae5df288475aa63ddae | CHANGES: 3 MAJOR (fixed) |
| Spec | 2 | 9b306f1b27f7cb5bce4eeae32d4c6e9eee1ae183:13165b3a9e4938a42528aae5df288475aa63ddae | CHANGES: 1 MAJOR (fixed) |
| Spec | 3 | 31cbda77df6487d072050ff54a55b6de41868a3e:13165b3a9e4938a42528aae5df288475aa63ddae | CLEAN (1 MINOR: agent-initiated provider resume, left to clarify/plan) |
| Plan | 1 | a0ffca5dedb4e146bf5ab61d75fca3fcbc39b919:6a7d599bd7d6d380ce4c8afc405b01095c336d91 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Plan | 2 | 7a0314a89407314e315a9bc5c4df676b7896a95f:6a7d599bd7d6d380ce4c8afc405b01095c336d91 | CHANGES: 1 MAJOR (fixed), 2 MINOR (fixed) |
| Plan | 3 | f7336d39da2163aeb5245509456a3a91207ec911:6a7d599bd7d6d380ce4c8afc405b01095c336d91 | CLEAN (2 MINOR, fixed, prose only) |
| Tasks | 1 | 88f01682a9aa66bc17d3ab75057ec889afc7f777:0b295551eba2928d4ff1de381dbd709c9c9e17f3 | CLEAN (3 MINOR, fixed; analyze 5 MEDIUM fixed or spec prose clarified) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
