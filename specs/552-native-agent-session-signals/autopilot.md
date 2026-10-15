# Autopilot ledger — #552 native-agent-session-signals

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Replace name-based hiding of agent work with the AI CLI's native hooks (issue #552); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #552
- **Worktree branch**: feat/552_replace-name-based-hiding-of-agent-work-with-the
- **Started**: 2026-10-12
- **Phase**: milestone M4 (gate green, reviews done, PR open)
- **Next step**: wait for CI and merge M4, then close

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #786 | design (spec, plan, tasks) | merged | 31a433b352e7301c5c860ea0cf64ca0c2f78eb1f |
| #795 | M1 every worktree listed | merged | 4ba66482fc44ac820233512a21001fe2e8a5bbef |
| #823 | M2 Claude sub-agent runs recorded, protocol 43 | merged | 6a8aacad57cd3a8648d6f1fa7cebbafa232dd83b |
| #870 | M3 provenance and attach_worktree removed, protocol 46 | merged | 5a65464712ecce80c7aed79475c585912a54399d |
| #881 | M4 provider gaps, provider table, superseded check | in review | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | every worktree listed, toggle and client name-based visibility removed, hook captures | #795 | merged |
| M2 | T010–T021 | full | Claude sub-agent runs recorded from SubagentStart/Stop, protocol 43 | #823 | merged |
| M3 | T022–T029 | full | provenance and attach_worktree removed, protocol 46 | #870 | merged |
| M4 | T030–T033 | full | Copilot mapping or gap, provider table, docs check | #881 | in review |

## Decisions

- Clarify round 1: FR-009 attach worktree flow removed with provenance (no remaining effect), session discovery/resume kept; FR-011 flat display until #551. Both agent-resolved, no escalation.
- Tasks: the plan's single 40 -> 41 protocol bump is split by milestone (M2 adds `SessionSummary.agents` -> 41, M3 removes attach shapes -> 42): each wire change moves the schema hash and lands in its own PR. Plan amended in intent; schema_hash pin follows per milestone.
- Tasks: M1 holds US1 + the T001 capture (Setup/Foundational); provenance/attach removal (FR-009/010) is its own milestone M3, the story-less removal, to stay under ~800 lines per PR; user-guide tasks sit in the milestone that ships the behaviour.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
- M2 bump: main was already at protocol 41 when M2 rebased, so `SessionSummary.agents` lands as 42 (M3's attach removal is then 43).

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 718ac3439483d010a1c175c99413ec4819148efd:13165b3a9e4938a42528aae5df288475aa63ddae | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec | 2 | ee648d9490a289c67416680a6aa1e02ecb62ea88:13165b3a9e4938a42528aae5df288475aa63ddae | CLEAN (2 MINOR, fixed, prose only) |
| Plan | 1 | f1bf590d11c47941fedf49c5d8d3f426b56a42b7:9122c7468c3de1adb70936b5209542842bf94f0a | CHANGES: 1 MAJOR, 3 MINOR (all fixed: spec edge cases and entity amended to the R1 model, test rows, stale-run rule, T001 capture first) |
| Plan | 2 | 9a3327187d96db4c03a1ac57711a4e95505eda13:1fc077b2c21c4f566e96e7ff6235834474d828af | CLEAN (1 MINOR, fixed, prose only) |
| Tasks | 1 | f33bebd86a11c339e1406fdfe83e8d36a17db03d:b55b6b77658c2c01450c00ef3ba20b6f0d2c81aa | CHANGES: 1 BLOCKER, 1 MAJOR, 3 MINOR (all fixed: M1 now removes only the client's reads of the classification; core deletion moved to T024; tests for 42 added to T022) |
| Tasks | 2 | 73f82d103e50af33acec2be9f1876de5ea25743b:4d47d38bec3db367185caa3f02b8a8d7dd6d369f | CLEAN |
| Diff A (code-review high) M1 | 1 | f56c31b32ec335c0bdd823ff595786a1465046db:9f92c4dc32aca6d3f2da21b9696ee7f1ecb2c94d | CHANGES: 1 MAJOR (fmt, fixed), 3 MINOR (stale docs, fixed); coordinator supplement: core strings fixed, daemon list_worktrees include_hidden noted into T024, claim item `false` kept to M3, per-worktree availability fan-out accepted, user-guide wording fixed |
| Diff B (conformance, sonnet) M1 | 1 | f56c31b32ec335c0bdd823ff595786a1465046db:9f92c4dc32aca6d3f2da21b9696ee7f1ecb2c94d | CLEAN (3 MINOR, fixed/none); Verify: worktree_visibility 7 passed, store_roundtrip 37 passed |
| Diff A (code-review high) M2 | 1 | 0434104615196360d0d89b3d96283a701ad8ba5f:55510b15508ff8a97204e57a080d28b1eaba66cf | CLEAN (3 MINOR: eviction no longer drops the new run and field length bounded, fixed; late start after stop not reconciled until restart, accepted) |
| Diff B (conformance, sonnet) M2 | 1 | same | CLEAN (3 MINOR: Verify queued behind build lock, covered by the full gate; no cycle-log kept for this feature; body-not-logged claim in a test comment, accepted) |
| Diff A (code-review high) M3 | 1 | 1a1b0a3d8085a87152e9cc529d52daa901a25ad7:f1be454fc6e0888a59192a4b3da446d23dbb086e | CLEAN; 1 MINOR (AttachDiscover doc), fixed |
| Review B (conformance sonnet) M3 | 1 | same | CLEAN; Verify passed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## M3 decisions

- `AttachTarget::Worktree` kept (resumes a stored session in its worktree); `WorktreeClaim` removed with provenance; list_worktrees no longer accepts `include_hidden`; banner offers "Resume all" (stored sessions) else "Review".

## Follow-ups not done

None.

## M4 decisions

- T001 found no installed Copilot CLI and no sub-agent event (research R8), so Copilot is a stated gap: T030 became a test that a Copilot event log never gains runs; T031 skipped.
- T033 also marks 021, 024, 025, 028, 033 and 575 (they mention the toggle) with a "Partly superseded" note; the script's list is derived by grep plus the three named specs.

## M4 follow-ups from review A (code-review ran on HEAD = M3 commit, findings are M3 code, not fixed here)

- runs.rs clean_half_created lost its app-created guard (deletes hand-made folder at the derived name on restart); stale doc comment.
- attach.rs offer_visible dropped the user_created_worktrees gate: Resume banner every launch.
- list_worktrees rejects a stale `include_hidden` argument.
- Resume all re-runs provider discovery per item; dead `created_worktrees`/`provenance_migrated` store fields; unused `_focused` param in attach_dialog; stale ClaimRequested doc in worktree.rs.
