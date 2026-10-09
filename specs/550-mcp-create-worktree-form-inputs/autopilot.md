# Autopilot ledger — #550 mcp-create-worktree-form-inputs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 550 (issue #550: MCP create_worktree should support the same inputs as the New worktree form (type, ticket, GitHub issue)); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #550
- **Worktree branch**: feat/550_mcp-create-worktree-should-support-the-same-inputs
- **Started**: 2026-10-08
- **Phase**: milestone M3 (gate green, PR open)
- **Next step**: orchestrator merges the M3 PR; then close.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #638 | Design (spec ships in it) | merged | 71d7195264cd757f40baf342ca3052ff7d541d8e |
| #640 | M1 | merged | ec1029040d9873e913c7bea231ceae2de8294978 |
| #648 | M2 | merged | 23e2db0e4c17ba6e2b7d28893d0999a9482cd0cf |
| #664 | M3 | open | - |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001-T009 | full | Derived type/ticket/name creates form's worktree; literal unchanged | #640 | merged |
| M2 | T010-T012 | full | Refusals and collision hint | #648 | merged |
| M3 | T013-T017b, T019-T021 | full | github_issue (M1 refuses it as not yet supported) | #664 | gate green, PR open |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| M3 review A (code-review high) | 1 | 146971b76647cbb0ca47e968c6954ce581f9373d:23e2db0e4c17ba6e2b7d28893d0999a9482cd0cf | 8 findings: stale tool description text and misplaced doc comment fixed; rest declined (below) |
| M3 review B (conformance) | 1 | same | CLEAN (1 MINOR = the doc comment, fixed) |
| M2 review A (code-review high) | 1 | befafbebc762204b57527b9eed16963c0657e4ab:13fd79b788db693ec37a762b4692dd5f1e1a38b0 | CLEAN (2 MINOR, not fixed: no hint on a lost race; test flake risk unproven) |
| M2 review B (conformance) | 1 | same | CLEAN, Verify 23 passed |
| Spec 550 | 1 | f31f875a52be3358279732b0a75b9977257c181c:99a3408db8a1672da2f13b0e7f9528a6525f6d6c | CHANGES: 1 MAJOR, 3 MINOR (fixed; MAJOR re-reviewed) |
| Spec 550 | 2 | 3106c30901864866deda136a173dde76eda2c26f:99a3408db8a1672da2f13b0e7f9528a6525f6d6c | CLEAN (1 MINOR, fixed) |
| Plan 550 | 1 | 9627256cc0e1e9592a0f44b0ea4d28d51d9fb12f:eec3ae7f613a1dd6bd22aafa9f79034f8063ae77 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Plan 550 | 2 | 726fd826ac5bdbfa011ae9879bf1eb8da387c6b3:eec3ae7f613a1dd6bd22aafa9f79034f8063ae77 | CHANGES: 1 MAJOR (directory-first collision order), 1 MINOR; fixed, checked against worktree.rs:952 and git.rs:24 |

## T001 baseline (SC-004)

The baseline run raced with my first edits (daemon lib did not compile mid-edit), so it is not a clean pre-change record. Shipped `create_worktree` tests were only reshaped for `Operation::CreateWorktree(CreateWorktreeRequest::Literal{..})` and all pass after the change (`mcp_create_worktree` 19 pass, `mcp_tools_catalog` 40, `mcp_policy`, `naming` 37). `github_locate_desktop_launch` fails on this host (gh only under mise); unrelated, run with `MICOLD_SKIP_GH_LAUNCH_TEST=1`. M1 was written implementation-first, then tests: no red-phase evidence (disclosed).

## M1 reviews

Review A round 1 (snapshot b7fd89c1...): F1 MAJOR empty ticket refused -> fixed (optional_text, tests added). Review B (sonnet): CLEAN. Full gate green (MICOLD_SKIP_GH_LAUNCH_TEST=1, host-only gh path issue).

## Decisions (plan)

GitHub lookup, label mapping and naming already live in micold-core; lookup runs in the daemon (research D1-D3). Added to core: IssueLookup single-issue read, naming_for_issue. Mapping read from settings store at call time.

## Declined review findings

- M3 A F7 (blank explicit ticket erases issue number): FR-006 says an explicit value replaces; the form lets the user clear the ticket after a pick. Kept.
- M3 A F4 (label_mapping file read under state lock): small file; persist_service_settings already does store IO under that lock. Kept.
- M3 A F3, F5, F6, F8 (refactor/style, label limit equals listing's 20, lookup before local checks): kept; no behaviour defect.
- Note: no policy rule denies create_worktree today, so the 'denied caller before lookup' test (T019) is covered structurally (check_policy precedes resolve_naming) plus a no-lookup test for calls refused at parse.
- Review A F2/B F1 (MINOR, audit target falls back to ticket/type): ticket is a reference naming the target, not free input text; kept.
- Review A F3 (MINOR, literal replay test thin): the shipped literal tests still cover mode/remote shapes; kept.

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
