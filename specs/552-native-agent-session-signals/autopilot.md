# Autopilot ledger — #552 native-agent-session-signals

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Replace name-based hiding of agent work with the AI CLI's native hooks (issue #552); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #552
- **Worktree branch**: feat/552_replace-name-based-hiding-of-agent-work-with-the
- **Started**: 2026-10-12
- **Phase**: tasks
- **Next step**: analyze, tasks review, gate, design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | every worktree listed, hiding and toggle removed, hook captures | | pending |
| M2 | T010–T021 | full | Claude sub-agent runs recorded from SubagentStart/Stop, protocol 41 | | pending |
| M3 | T022–T029 | full | provenance and attach_worktree removed, protocol 42 | | pending |
| M4 | T030–T033 | full | Copilot mapping or gap, provider table, docs check | | pending |

## Decisions

- Clarify round 1: FR-009 attach worktree flow removed with provenance (no remaining effect), session discovery/resume kept; FR-011 flat display until #551. Both agent-resolved, no escalation.
- Tasks: the plan's single 40 -> 41 protocol bump is split by milestone (M2 adds `SessionSummary.agents` -> 41, M3 removes attach shapes -> 42): each wire change moves the schema hash and lands in its own PR. Plan amended in intent; schema_hash pin follows per milestone.
- Tasks: M1 holds US1 + the T001 capture (Setup/Foundational); provenance/attach removal (FR-009/010) is its own milestone M3, the story-less removal, to stay under ~800 lines per PR; user-guide tasks sit in the milestone that ships the behaviour.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 718ac3439483d010a1c175c99413ec4819148efd:13165b3a9e4938a42528aae5df288475aa63ddae | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec | 2 | ee648d9490a289c67416680a6aa1e02ecb62ea88:13165b3a9e4938a42528aae5df288475aa63ddae | CLEAN (2 MINOR, fixed, prose only) |
| Plan | 1 | f1bf590d11c47941fedf49c5d8d3f426b56a42b7:9122c7468c3de1adb70936b5209542842bf94f0a | CHANGES: 1 MAJOR, 3 MINOR (all fixed: spec edge cases and entity amended to the R1 model, test rows, stale-run rule, T001 capture first) |
| Plan | 2 | 9a3327187d96db4c03a1ac57711a4e95505eda13:1fc077b2c21c4f566e96e7ff6235834474d828af | CLEAN (1 MINOR, fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
