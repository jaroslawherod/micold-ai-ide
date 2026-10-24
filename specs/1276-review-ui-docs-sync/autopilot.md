# Autopilot ledger — #1276 review-ui-docs-sync

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1276 "Review UI: docs, layout snapshot, visual pass and mockup sync" (body and owner comment quoted in spec.md Input); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1276
- **Worktree branch**: claude/project-thread-kxels1
- **Started**: 2026-10-24
- **Phase**: design PR
- **Next step**: orchestrator waits on design PR CI and merges; then M1 (blocked on #1269–#1275)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | guide describes the Review tab; `review_keys_doc` holds keys = guide | — | blocked: #1269–#1275 |
| M2 | T014–T025 | full | fixture covers every review state, re-recorded once; `review_tab_fits` gate | — | blocked: M1, #1269–#1275, #1128 |
| M3 | T026–T029 | full | visual pass 17 checks × 2 themes at 1280×800 with verdicts | — | blocked: M2, #1269–#1275, #1128 |
| M4 | T030–T032 | docs | mockup canvas (incl. Overview board, `OverviewCard`) matches M3 crops | — | blocked: M3, #1269–#1275, #1128 |
| M5 | T033–T034 | docs | `review-redesign.md` is a record | — | blocked: M1, M4, #1269–#1275, #1128 |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-004: is the guide-equals-bindings check the micold-core logic + unit tests clause? | Yes: micold-core test compares a marked shortcut table in reviewing-changes.md with the bound review key table; page gets -micold-docs | agent | crates/micold-core/tests/terminal_only_doc.rs; .gitattributes:125 |
| 2 | clarify | FR-009: visual pass widths | 1280×800 only, both themes; other widths via FR-007 gates | agent | review-redesign.md#Tasks; issue acceptance |
| 3 | clarify | FR-012: unbuilt mockup elements removed or labelled? | Doc-listed follow-ups (split option, FR-013 list) stay labelled "follow-up, not built"; other unbuilt elements removed | agent | review-redesign.md#0 ("Split option … Follow-up") |
| 4 | tasks | M1 has 13 tasks, over the ~10-task split threshold: split? | No: M1 is mostly docs plus one test, under ~800 lines | agent | tasks review round 1 F2 |
| 5 | tasks | M2 has 12 tasks, over the ~10-task split threshold: split? | No: M2's single fixture re-record (FR-005) cannot be split across PRs | agent | tasks review round 1 F2; spec FR-005 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | cd6df384829f4a9cf135e66858ff627637928a05:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CHANGES: 3 MAJOR, 2 MINOR (run by orchestrator); all fixed |
| Spec | 2 | e130e157ec45aa2c55f68f1cd624e18698a09441:7783f026de44f8644464cd7933653c0bed11d5c1 | CLEAN, 2 MINOR (sonnet): FR-005 groups reworded to match US2 sc.1 — fixed, prose only; FR-009 check list wrapped — fixed, prose only |
| Plan | 1 | a39dc72ffead95575b27b0702260cb2dd3dc760a:428a3f5fc565c1a7898b2b37503471f52e1f5e0a | CHANGES: 3 MAJOR, 3 MINOR (run by orchestrator); all fixed: F1 Pull requests section rewritten in outline + link text + §A2 grep; F2 review_tab_fits calls status-bar and session-area checks made pub(crate); F3 core table is a requirement on #1274, blocker issue if missing, no move here; F4 session_area_stability press tests named; F5 stub `gh` in visual-pass world; F6 thinner-table fallbacks in shortcut contract |
| Plan | 2 | 82562df08a1ccd946a30c12d3291e816b721a0b9:be1df5925e6a15b503e9550140c38e290b0a4075 | CHANGES (sonnet): F2–F6 fixes verified; 1 MAJOR: §A2 `Changes view` grep would still hit settings.md:653/659, worktrees-and-sessions.md:978, agent-tools.md:299 — fixed: every hit listed in outline § Inbound links and plan §3 as reworded to the Review tab |
| Plan | 3 | b071908fcf6dd7d0c516e4321efa5f0e7d2ee778:c8b7b7954d99ef1165f82f84f9e53ed1e8e5354a | CLEAN, 1 MINOR (sonnet): parallel-runs.md:84,91,92, docs/README.md:45, layout-snapshot.md:105 also say "Changes view" — fixed, prose only: added to outline § Inbound links and plan §3 as reworded to the Review tab |
| Tasks | 1 | 2172de0899146cb46d218285f19e84958b64c45d:07827f22ca321e5ed423241daeacccd0ba41a4e7 | CLEAN, 3 MINOR (session model, run by orchestrator; speckit-analyze could not run, reviewer checked cross-artifact consistency in its place); all fixed: F1 checklist items 16–17 ticked, notes say clarify answered all three; F2 M1/M2 size kept, decisions 4–5; F3 T005 names the `#[path]` include of marked_table.rs, M1 Verify adds `review_sheet_doc` |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Environment: `gh` is not authenticated and `mise` is missing in this cloud container; GitHub reads
  went through the GitHub MCP tools. Blockers #1269–#1275 and #1128 are unmerged at spec time; the
  spec describes their deliverables from their issue bodies and `docs/development/review-redesign.md`.
- `speckit-analyze` is not available as a skill in this container (Skill tool: unknown skill); the
  tasks unit checked FR/SC → task coverage and milestone dependency direction by hand, and the
  tasks reviewer covers the rest. Branch rebased onto origin/main 9c0b8108 at the tasks unit start.
