# Autopilot ledger — #550 mcp-create-worktree-form-inputs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 550 (issue #550: MCP create_worktree should support the same inputs as the New worktree form (type, ticket, GitHub issue)); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #550
- **Worktree branch**: feat/550_mcp-create-worktree-should-support-the-same-inputs
- **Started**: 2026-10-08
- **Phase**: tasks
- **Next step**: commit and open design PR; then M1.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #638 | Design (spec ships in it) | open | - |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001-T009 | full | Derived type/ticket/name creates form's worktree; literal unchanged | - | pending |
| M2 | T010-T012 | full | Refusals and collision hint | - | pending |
| M3 | T013-T017b, T019-T021 | full | github_issue (M1 refuses it as not yet supported) | - | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec 550 | 1 | f31f875a52be3358279732b0a75b9977257c181c:99a3408db8a1672da2f13b0e7f9528a6525f6d6c | CHANGES: 1 MAJOR, 3 MINOR (fixed; MAJOR re-reviewed) |
| Spec 550 | 2 | 3106c30901864866deda136a173dde76eda2c26f:99a3408db8a1672da2f13b0e7f9528a6525f6d6c | CLEAN (1 MINOR, fixed) |
| Plan 550 | 1 | 9627256cc0e1e9592a0f44b0ea4d28d51d9fb12f:eec3ae7f613a1dd6bd22aafa9f79034f8063ae77 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Plan 550 | 2 | 726fd826ac5bdbfa011ae9879bf1eb8da387c6b3:eec3ae7f613a1dd6bd22aafa9f79034f8063ae77 | CHANGES: 1 MAJOR (directory-first collision order), 1 MINOR; fixed, checked against worktree.rs:952 and git.rs:24 |

## Decisions (plan)

GitHub lookup, label mapping and naming already live in micold-core; lookup runs in the daemon (research D1-D3). Added to core: IssueLookup single-issue read, naming_for_issue. Mapping read from settings store at call time.

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>

## Tasks review

- analyze: 0 CRITICAL/HIGH; C1-C5 fixed (tasks, contract). Declined: C7-C10 (prose/plan nits, LOW).
- Review round 1 (snapshot 04276a7d198a9fcc175ede45f27a434d9d112e6e:091c5a0defd667af6b269ec6b10d2e41fa445ac4): F1 checklist ticked, F2 github_issue guard added to M1, F3 T010 reworded as characterisation, F4 T018 folded into T020, F5 recorded here. Structure-only fixes; checked by me.
