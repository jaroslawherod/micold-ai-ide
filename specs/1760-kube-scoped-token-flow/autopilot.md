# Autopilot ledger — #1760 kube-scoped-token-flow

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1760: Scoped-token step 11: assert the credential flow in a kind (Kubernetes) pod; labels: none originally, now flow:feature (switched from chore)
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1760
- **Worktree branch**: test/1760_scoped-token-step-11-assert-the-credential-flow-in
- **Started**: 2026-11-12
- **Phase**: tasks
- **Next step**: design PR merge, then M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017, T020 | full | tokens delivered to pod Secret and daemon.toml behind the opt-in (setting only via daemons.json) | - | pending |
| M2 | T018–T019 | light | form tick and user guide | - | pending |
| M3 | T021–T027 | full | kube_real_scoped_token and forge harness | - | pending |

## Decisions

- FR-008: per-daemon opt-in for delivering GitHub tokens into a cluster Secret, off by default (decided by user, clarify round 1).
- FR-009, FR-010: agent-resolved from the repo (see spec Clarifications).

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec review | 1 | spec.md | c1af633993e0d08fce04690906bf0074fe12df1d | dc49940e4d330da0d81222db5812aa2e88ce4a13:c1af633993e0d08fce04690906bf0074fe12df1d | CHANGES: 1 MAJOR declined (open markers go to clarify), 3 MINOR fixed |
| Plan review | 1 | plan.md, research.md, data-model.md, contracts | 2ea1951e53ce0daa188acf55cc01f06b058a2534 | e5bec1b3666ceed4478d5c774ad949d9017a7fac:2ea1951e53ce0daa188acf55cc01f06b058a2534 | CHANGES: 2 MAJOR fixed (runtime-project source narrowed to saved projects; KubeEnv.tokens seam), 3 MINOR fixed |
| Plan review | 2 | fix diff | 2ea1951e53ce0daa188acf55cc01f06b058a2534 | fbbd354fff65ccc938db0d8d610033b6124a24d6:2ea1951e53ce0daa188acf55cc01f06b058a2534 | CLEAN (3 MINOR: 1 fixed, 2 left for tasks) |
| Plan review | 2 | plan fix diff | 44e5569db6c05e6b2c3da07cd58d488222b8f023 | fbbd354fff65ccc938db0d8d610033b6124a24d6:2ea1951e53ce0daa188acf55cc01f06b058a2534 | CLEAN |

| Tasks analyze | 1 | tasks.md | - | - | 1 HIGH (arm C vs AS3) and 4 MEDIUM (task order, M1 docs vs M2 tick, mount vs home, test-first notes) fixed; prose/task-list only |
| Tasks review | 1 | tasks.md, milestones | f8a94ddf6 | 893ed26f20eb272e8a1574de8cdaf6fe034c8fdd:f8a94ddf620220ccd11a592b6644bb6b2a5a3d57 | CHANGES: 1 MAJOR fixed (user-guide task T020 moved to M1), 2 MINOR fixed; prose/task-list only, no re-review |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| spec | spec review r1 | F1 MAJOR: three open markers (FR-008..010) leave scope unsettled | They are the template's allowed three; the rubric sends each to clarify. FR-008 is escalated there. Fixed F2, F3, F4 (prose/edge case). |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- docs/user-guide/sandboxed-daemon.md promises a sandbox pushes with a pasted token, but no client code delivers `[[github.tokens]]` to a container (deliver_container) either; separate follow-up.
