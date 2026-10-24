# Autopilot ledger — #1186 jira-advanced-jql

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1186 — issue #1186 "Support advanced JQL in the Jira issue selector"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1186
- **Worktree branch**: claude/project-thread-vf2ck6
- **Started**: 2026-10-24
- **Phase**: M1 PR
- **Next step**: orchestrator opens the M1 PR from branch claude/project-thread-vf2ck6 (title/body in the M1 unit's scratchpad, label docs-not-needed), waits on CI and merges; then milestone M2 (T013–T017)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1346 | design (spec, plan, tasks) | merged | 527d91b36196fda8950f2d3e1d6a193fe3a0f1d9 |
| (to open) | M1 core rules + advanced request | pushed, PR not opened (gh unauthenticated) | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012 | full | Core rules + advanced request (no UI) | (to open) | gate green, pushed |
| M2 | T013–T017 | full | Render-free selector state and the run (no view yet) | — | planned |
| M3 | T018–T023 | full | Basic/Advanced switch, query field, results, pick | — | planned |
| M4 | T024–T032 | full | Jira's error messages and Retry | — | planned |
| M5 | T033–T039 | full | Remember last query and mode per project | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D7 | tasks | How to cut US1 (≈1,300 lines: core, adapter, reducer, shell, view)? | Three milestones M1 core+adapter, M2 render-free state, M3 view+guide; M1/M2 keep the UI unreachable (milestones rule 6) and carry `docs-not-needed`; no acceptance-scenario split exists because every US1 scenario needs the switch and field. T040 (quickstart §B record) is code-free Polish → close unit | agent-resolved | plan.md#delivery-order; tasks.md#milestones |
| D6 | plan | Where are the mode and last run query remembered? | Owner-only `<data_dir>/jira/<project id>.query.json` beside the token, deleted with the connection; not the project state file, not settings.json | agent-resolved | research.md R7 |
| D5 | clarify | Remembering and source switch | Mode remembered at each switch, query at each run; GitHub↔Jira keeps mode, resets draft to last run query, re-runs it in advanced (FR-003a) | agent-resolved | 533 FR-017; #1185 FR-016 |
| D4 | clarify | Open in advanced with no last run query | Nothing sent; prompt in empty input; basic list loads only on first switch to basic | agent-resolved | 533 FR-020 |
| D3 | spec | How does the code compose with #1185's changes to search_jql/list_jql? | Plan adds a separate advanced-JQL builder beside search_jql; search_jql/list_jql are not restructured (if the plan finds it must, record it here and tell the orchestrator) | agent-resolved | coordinator note 2026-10-24; specs/1185-jira-selector-filters on origin/claude/project-thread-xwkotj lists #1186 as out of scope |
| D2 | spec | What does an empty advanced query list? | `project = "<KEY>" ORDER BY updated DESC` (no status filter, same rule as any advanced query) | agent-resolved | spec review round 1 F6 |
| D1 | spec | How does advanced mode compose with #1185's basic filters? | Basic/advanced toggle: advanced hides and ignores search text and basic filters; switching back restores them | agent-resolved | orchestrator scope note; spec.md#Assumptions |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| speckit-analyze | 1 | — | 0 CRITICAL/HIGH; 4 MEDIUM fixed (T040 left to close unit per milestones rule 4; discard cases in T013/T037; FR-016 scan of ui in T021; T022/T030 Failed split); LOW C3/C4 fixed; plan delivery order now points to tasks.md §Milestones (prose only) |
| Tasks review | 1 | 8727f9ed09cbf4af667def1d20aeddebf03c9b61:064906ac611aee3457b2f71f5f6823942102df0d | CHANGES: 1 MAJOR (M3 Verify named non-existent target; now `--test layout_snapshot issue_rows_show_all_text`), 2 MINOR fixed |
| Tasks review | 2 | 82bd1f0c1bca2c86439cdeeff0b3a1bc9f62a34a:064906ac611aee3457b2f71f5f6823942102df0d | CLEAN (1 MINOR fixed: T033 [P] dropped) |
| Spec review | 1 | 3f9cc1e70535abc53505245bbebd94feceb8cdb5:b43218cdb7025279d1129ac4f6452b88298f123b | CHANGES: 3 MAJOR, 3 MINOR — all fixed (quote/sort-clause terms; FR-004a auto-run on open; draft vs last run query; moved-issue edge; HTTP 400 = invalid query; empty query = same rule as sort-only) |
| Clarify | 1–2 | — | CLEAN: round 1 five questions agent-resolved (1 deferred to plan); round 2 no critical ambiguity |
| Plan review | 1 | 840893a6b7c27c77bf984af45f25a08d59317ea2:57116fa3b1d2a80ddd0ef5798f90e39ed94bd94b | CLEAN (3 MINOR, fixed, prose only: awaited_jira_query accessor named; data-model §3 ref; FR-016 source scan added) |
| Spec review | 2 | 50939e7590e958b50053133db00b8fe7f26e966b:5510e0471524f4d4a847f1d9f80dc643b9b99499 | CLEAN (1 MINOR kept: HTTP 400 in FR-010 is the observable rule round 1 asked for) |
| M1 review A (high) | 1 | 9f945fcd9f9ecead6d6fc3aed002bbdf583fd556:d74375825a41dbcd2302e960f7693f17d1438490 | CLEAN (1 MINOR fixed: an `ORDER BY` inside parentheses split a group; sort clause now only at depth 0, contract §2 updated, test added) |
| M1 review B (sonnet) | 1 | 9f945fcd9f9ecead6d6fc3aed002bbdf583fd556:d74375825a41dbcd2302e960f7693f17d1438490 | CLEAN (same MINOR as A, fixed); Verify: jira_query 25, jira_load 8, jira_http_adapter 18 passed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
