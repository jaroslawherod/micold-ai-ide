# Autopilot ledger — #1186 jira-advanced-jql

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1186 — issue #1186 "Support advanced JQL in the Jira issue selector"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1186
- **Worktree branch**: claude/project-thread-vf2ck6
- **Started**: 2026-10-24
- **Phase**: M4 PR
- **Next step**: M4 done on local branch 1186-m4-wip (full gate green at b6223f6f; review A and B clean; visual pass B4–B6 PASS, screenshots in /mnt/project-files/issue-1186/m4-*); after #1379 merges the orchestrator rebases onto main, re-gates, pushes and opens the M4 PR (title/body in the unit's scratchpad); then milestone M5 (T033–T039)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1346 | design (spec, plan, tasks) | merged | 527d91b36196fda8950f2d3e1d6a193fe3a0f1d9 |
| #1350 | M1 core rules + advanced request | merged | 3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 |
| #1364 | M2 render-free selector state and the run | merged | 5b5064bc72729361327aabc63581311b92d0815f |
| #1379 | M3 Basic/Advanced switch, query field and results | open, CI running | — |
| (to open) | M4 Jira's error messages and Retry | full gate green at b6223f6f, local branch 1186-m4-wip, not pushed | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012 | full | Core rules + advanced request (no UI) | #1350 | merged |
| M2 | T013–T017 | full | Render-free selector state and the run (no view yet) | #1364 | merged |
| M3 | T018–T023 | full | Basic/Advanced switch, query field, results, pick | #1379 | open, CI running (rebased onto main at e545d5f5) |
| M4 | T024–T032 | full | Jira's error messages and Retry | (to open) | full gate green at b6223f6f, not pushed (based on e545d5f5) |
| M5 | T033–T039 | full | Remember last query and mode per project | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D11 | M4 | Where do a failed run's lines and **Retry** sit, when the form has 5 px left before the app bar? | The lines (Jira's messages, or basic mode's notice) scroll in a one-line viewport (20 px) under the field, as the picked issue's details scroll (837); **Retry** sits in the field's row after **Run** | agent-resolved | panel_placement red with two message lines (dialog at y=60) and with Retry on its own row (y=48); containment exemption `QUERY_FAILURE_CONTENT`; contracts/selector-ui.md §1 updated |
| D10 | M3 | Where does the Basic/Advanced switch sit? | At the top of the issue picker's column (XS gap), not as a dialog field after `provider_switch`: as a field it added 48 px and pushed every Jira dialog's card to y=60–64, inside the app bar (65); in the picker it adds 36 px and the card stays below the bar | agent-resolved | `the_query_mode_switch_keeps_the_panel_placement` red with the switch as a field; contracts/selector-ui.md §1 updated |
| D9 | M2 | Where does the client's `jira_query` module live? | `features/worktree_form/jira_query.rs` (`features::worktree_form::jira_query`), a submodule of the form as `settings/github_tokens.rs` is of settings, not `features/jira_query.rs`: `tests/feature_registration_cost.rs` makes every file in `features/` a feature (own isolation test, own `update(&mut State, Msg)`, no naming of `worktree_form`), and this state is the form's, applied with the form's seq. Plan, data model, research and tasks paths updated (prose only) | agent-resolved | gate run 2: feature_registration_cost 3 FAILED (`jira_query` names `worktree_form`; no isolation test; no `update`) |
| D8 | M2 | How does an advanced row pick reach `apply_pick` with the label mapping, which only the shell reads? | `jira_query::Msg::RowPicked(i)` is resolved by the shell exactly as `IssueRowPicked`: `State::issue_number_at` (now reading `shown_issues()`, the advanced results in advanced mode) gives the number, then `IssuePicked { number, mapping }`, whose reducer finds the issue in `shown_issues()` and calls `WorktreeForm::apply_pick`. One pick reducer for both modes, so FR-009's "exactly as basic" holds by construction | agent-resolved | contracts/selector-ui.md §5; tests/jira_query_state.rs `an_advanced_pick_fills_the_form_as_a_basic_pick_of_the_same_issue`; main_tests `jira_query_run` |
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
| M2 review A (high) | 1 | e62354d4a4924fa030171958c744b15b81a486ae:5cdbf8a3e887c5d7851318adbee7eba5b87b7f7c | CLEAN (2 MINOR fixed: `IssueRetry` no longer loads a failed basic list while advanced mode shows; a pick's title is looked up in both lists, so the default prompt keeps it across a switch; tests added) |
| M2 review B (sonnet) | 1 | e62354d4a4924fa030171958c744b15b81a486ae:5cdbf8a3e887c5d7851318adbee7eba5b87b7f7c | CLEAN; Verify: bin 709, jira_query_state 18, issue_source_state 49, jira_source_state 20, issues_are_requested… 3, jira_is_requested… 6 passed |
| M3 review A (high) | 1 | 05b5792f2b197c5d37170e31d18c562fe1668d83:9e7ccfdb7909b15402e98a4f3ad95432920f3940 | CHANGES: 1 MAJOR (editing the query kept the row highlight, so Enter picked a stale row instead of running; fixed in `DraftChanged`, test `editing_the_query_drops_the_highlight`), 1 MINOR fixed |
| M3 review A (high) | 2 | 158a6fb536deca830d63382549c53caefc87f7f1:a4eddeb72ddfa28cd1d169141c8fc4d7b03c752d | CLEAN (sonnet, fix diff: F1, F2 hold) |
| M3 review B (conformance, sonnet) | 1 | 158a6fb536deca830d63382549c53caefc87f7f1:a4eddeb72ddfa28cd1d169141c8fc4d7b03c752d | CLEAN (1 MINOR fixed: an empty result said "No issue matches this query." twice; the list's empty message dropped, the caption kept) |
| M3 visual pass (quickstart §B B1–B3, B7) | 1 | 1c12b691 binary | PASS all four (Xvfb, tiny-skia, fake_jira); not run: B1's GitHub-source variant (no remote in the scratch repo), B3 sort order (the fake ignores ORDER BY); the fake repeats its page because it always returns a next-page token, not a client defect |
| M4 review A (high) | 1 | 42865ee4f31bfa39fa99f5e3d0f7d8f1dda6ed39:8d16ddb3ad093ec30a9d914d907c8e59c6c5514d | CLEAN (1 MINOR declined: a 400's body is read on every path, not only the query's) |
| M4 review B (conformance, sonnet) | 1 | d2a6954e44c9ed190f293e3dd3d12fa3094acd31:b6223f6f5bdaeebc38a4e5f39b7f54a3e5dadc5e | CLEAN, no findings; Verify: jira_query 30, jira_http_adapter 22, jira_query_state 26, layout_snapshot 114 passed |
| M4 visual pass (quickstart §B B4–B6) | 1 | b6223f6f binary | PASS all three (Xvfb, tiny-skia, fake_jira `badjql`); screenshots /mnt/project-files/issue-1186/m4-*.png; noted, not defects: each run sends two requests because the fake always returns a next-page token (as in M3); one Retry click lost to xdotool on the slow renderer, the second worked |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M4 | A | `refusal_body` reads up to 64 KiB of every 400, also on the basic list and status bisection paths that drop it | T027 specifies `BadRequest(Vec<u8>)` read in `send`; a 400 body is a few hundred bytes and capped at 64 KiB, and a basic 400 is a failure path; splitting `send` per caller adds a second error shape for no observable gain |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
