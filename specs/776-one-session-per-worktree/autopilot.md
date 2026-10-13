# Autopilot ledger — #776 one-session-per-worktree

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 776 — "One session per worktree, chosen at start" (issue #776, verbatim in the issue); labels: enhancement, in-progress, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #776
- **Worktree branch**: feat/776_one-session-per-worktree-chosen-at-start
- **Started**: 2026-10-12
- **Phase**: amendment PR open; waiting for merge
- **Next step**: amendment PR merges; then milestone M2 (soft limit)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #791 | Design (spec, plan, tasks) | merged | 6b60f3201a2e4dacdfc8d7d626ea55384eb959a0 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | Second session refused by daemon (RPC, create_session, review send); wire fields + v41 | #794 | merged (#794) |
| M2 | T037–T042 | full | Soft limit: `allow_additional_session` on create_session, wire v42 (the start flag lands with the start block in M7) | | pending |
| M3 | T014–T016, T021 | full | create_worktree / WorktreeCreate define the session at start (WIP at tag wip-776-m3, cherry-picked after M2) | | pending |
| M4 | T017–T020, T022, T036, T043–T045 | full | Dialog fields; worktree rows only; row press opens the default session | | pending |
| M5 | T046–T049 | full | Row menu "Start another session..." with confirmation | | pending |
| M6 | T023–T030 | full | Resume replaces or keeps both; attach cap | | pending |
| M7 | T031–T035 | full | Several sessions intact; start beside a live one needs confirmation | | pending |

## Decisions

- Tasks: optional hooks (docguard review, tdd plan) skipped; test tasks are ordered first by hand. All wire additions and the v41 bump land in M1 so later milestones need no second bump. Foundational+US1 server half is 13 tasks; kept whole (no split gives an observable deliverable earlier). Polish has no code: quickstart results are left to the close unit.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | amend | Is the one-session limit absolute? | Soft: refusal naming the existing session stays the default; more than one session allowed when really requested; supersedes the issue's "no way to run more than one" and M1's hard refusal | user | "the limit should be soft, allow user to start more than on session if realy requested" |
| 2 | amend | Sidebar content and selection | Only worktrees by default; selecting a worktree selects its default session | user | "by default it should show only work tree and when selecting a worktree it should select default session" |
| 3 | amend | Name of the explicit request | boolean `allow_additional_session`, default false, on create_session/start_session and the wire messages; app: row menu entry plus confirmation | orchestrator recommended, agent-resolved | snake_case booleans/`ai_cli`, `prompt` conventions in core/src/mcp/tools.rs |
| 4 | amend | Default session | last viewed in this window if un-archived, else first un-archived in list order | agent-resolved | `Session` has no persisted activity timestamp; refusal already names the first in list order (occupancy.rs) |
| 5 | amend | Extra sessions' place | sub-rows of their worktree, only when it has several, collapsed by default; single-session rename/close in the worktree row menu | agent-resolved | ui/sidebar.rs already draws expandable session sub-rows |

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
| Code A M1 | 1 | b0bf3f11714bee25a5b75389dd46cfb3e8e9d57f:8a53a8576b8e2d739b53ace3b35d15c852c98337 | CHANGES: 1 BLOCKER (Display compile error), 1 MINOR (occupied send mapped to Refused); both fixed |
| Conformance B M1 | 1 | 592f90467d46ed5fd93eb67960a2dc5e4456973e:7bb8c03e4b755bd0b5faf262c0b63fe3bf4db1ba | CLEAN (2 MINOR: T008 path text fixed; per-lifecycle refusal cases covered by core occupancy unit tests) |
| Amendment (spec+plan+tasks) | 1 | 84499e08a0ccde4f8380eb9d9d7df6e33210f7b9:b361448a232e0df33ff72231f33f9712ded5cf5d | CHANGES: 2 MAJOR (start flag and attach assertion moved to M7/M6), 4 MINOR; all fixed |
| Amendment | 2 | 71db8a9058396eabe058cfa25dde64f9e1c5fd8e:b361448a232e0df33ff72231f33f9712ded5cf5d | CLEAN (2 MINOR fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. The soft-limit amendment is in the amendment PR (spec, plan, tasks, contracts only; no code).
The uncompiled WIP for old M2, now M3 (`create_worktree` with a definition; commits d860ad3d,
201eee28, docs T021) is kept off this branch at the local tag `wip-776-m3` (commit cf638754): the
M3 unit starts from `git cherry-pick d860ad3d 201eee28` after M2 merges, then fixes it to the
`Additional::No` signature. Core tests passed earlier; daemon tests never ran; no gate, no review B.
The M1 items that change are listed in research.md R12.

## Open escalation

None.

## Follow-ups not done

- Building without the repo build lock (MICOLD_NO_BUILD_LOCK=1, the lock queue was hours long) raced other worktrees on target-shared: one stale `micold-daemon` binary (v40) and one stale micold_core rlib failed tests that passed on rerun. Not a code fault.
- TDD red phase was not recorded for M1 (tests written with the implementation; no tdd/ dir).
