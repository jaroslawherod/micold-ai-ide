# Autopilot ledger — #1400 github-selector-filters

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1400; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1400
- **Worktree branch**: claude/project-thread-5zc5qk
- **Started**: 2026-10-27
- **Phase**: implement
- **Next step**: open the M1 PR (orchestrator; branch `1400-m1` pushed to `claude/project-thread-5zc5qk`, title/body in the scratchpad `pr-title-m1.md`/`pr-body-m1.md`), wait for CI and merge; then M2. T061 (Polish, no code) left to the close unit.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1558 | Design (spec, plan, tasks) | merged (rebase) | 1236152a3c3438f2bd27a213a897ec576ad397fd |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018 | full | Core filter rules, filtered load, label choices, remembered format (no UI) | pending (orchestrator opens) | reviews clean, full gate green at 522f25800 (`scripts/gate.sh`, 593 test results ok, scripts 0 failures), pushed |
| M2 | T019–T030 | full | Exclude and include labels in the selector (US1) | | implemented on branch `1400-m2` (stacked on M1 522f25800), scoped gate green; review round 1 (A, B, visual pass) pending |
| M3 | T031–T035 | full | Remember the basic filters per project (US2 AS1, AS2, AS5) | | planned |
| M4 | T036–T039 | light | Filter by state, Closed chip (US5) | | planned |
| M5 | T040–T049 | full | Advanced GitHub search query with its errors (US3, US4) | | planned |
| M6 | T050–T052 | light | Remember advanced mode and last run query (US2 AS3, AS4) | | planned |
| M7 | T053–T060 | full | Assignee, author, milestone (US6; provisional D1) | | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | Are assignee, author and milestone basic filters in scope (issue says "optionally")? | Yes: US6 as P3, built in the last milestone | provisional: recommended option, awaiting user | spec.md US6/FR-003/FR-005 already specify them; issue #1400 lists them in the basic-filter proposal; no repo source settles it. If the user picks "leave them out", a later unit moves US6 to Out of Scope. |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | a81cb29b681279bab9bac6fd165413021a0bd717:1132114243bde1ea489deb6c3c5ea3032f754e6f | CHANGES: F1 MAJOR (FR-002 vs Jira-and-back edge case: draft) fixed, draft kept; F2 MINOR (FR-013 empty last run query) fixed, a run empty query re-runs, prompt only when none ever run; F3 MINOR (FR-007 advanced mode) fixed, FR-007 and US1 s6 basic mode only |
| Spec review | 2 | ef51560eb3b59349331931d5814bbada52ed7dff:6929b5aec40be28081e16fd917d82c2cc3390bad | CLEAN (scoped re-review, sonnet): round-1 F1–F3 all fixed; F1 MINOR (Edge case 'Switching source to Jira and back': what reloads in advanced mode) fixed, re-runs the last run query, not the draft |
| Plan review | 1 | dca80a0048327e23892aea730395cc936f33c90d:300a7b70807ff667b356bcdc9dcd3528cc1373e4 | CHANGES: F1 MAJOR (filtered search beyond appends raw typed text under ISSUE_ADVANCED, `OR`/`)`/`repo:` escape the filter) fixed: typed text enters as quoted words, `None` when no word, search-rules §4 rows for `crash OR ui`, `a)`, `repo:x` + core tests, R4/data-model updated; F2 MINOR (Closed chip vs status-board status) fixed: Closed wins, not stale, selector-ui §3 + issue_rows_show_all_text arm |
| Plan review | 2 | 1df92c5dd1194b03f86ff53cb2d49ca2d9e61d4d:062d5a81eaec848b15a02f648cb52f8e2be7d71c | CHANGES (scoped, sonnet): round-1 F1 and F2 confirmed fixed; F1 MAJOR (typed `\` could close its own quote under ISSUE_ADVANCED, unverified) fixed: `\` dropped like `"`, §4 row `a\ OR b` + `None` rows, R4/data-model updated, backslash probe added to R7/M1 fixture probe; F2 MINOR (R4 Rejected list order a,c,b) fixed |
| Plan review | 3 | 7ef6a6ed83e2180a6e75229f426245b7253c762a:f3a47188b3059b604added461442c2a427b087ef | CLEAN (scoped re-review, sonnet): round-2 F1 and F2 confirmed fixed; no new contradiction |
| Tasks review | 1 | f5fa5bfd7b7859e302bfa962e54775ef046fef9e:7f0754f2ffcf14296b214ca93ee778cbfb417a13 | CHANGES: F1 MAJOR (M4 state change never remembered) fixed: T036 asserts `Remembered(Basic)` per state change + remembered Closed reopens with one `is:closed` listing, T038 emits it, M4 depends on M3, FR-021 (state) in Satisfies; F2 MAJOR (M7 US6 choices never remembered) fixed: T057 asserts `Remembered(Basic)` + `from_remembered` round trip, T058 emits it, FR-021 (US6) in Satisfies; F3 MINOR (M7 depends) fixed: M4, M6; F4 MINOR (checklist note) fixed, prose only: resolved as provisional D1. Dependencies section updated to match |
| Tasks review | 2 | 207cf7e50321622990b774afe49fb080ed3f61e5:f13cb90003f50fb93e8427f02f0a869de921b205 | CLEAN (scoped re-review, sonnet): round-1 F1–F4 all fixed; no new contradiction |
| M1 Review A + B | 1 | 71cbb1a04f82954925457fedd5089a6ad915a306:a3e43dbacff6b9601ba1acfd5bd86df98c94719f | A (code-review, high): CLEAN with 3 MINOR — F1 `LabelName::parse` accepted `\`, F2 scope check missed `(a)repo:`/`"x"repo:`, F3 FORBIDDEN/INSUFFICIENT_SCOPES at `["search"]` read as `Rejected`; all fixed test-first (`\` refused, scope checked after any char but alnum/`_`/`-`, both types → `Load(graphql_error_of)`; §2/§5/§6 rows, spec FR-006/Edge, data-model, research R4/R5/R6/R7 updated). B (conformance): CHANGES — F1 MAJOR no red-before-green evidence: fixed by `tdd/cycle-log.md` (red reconstructed structurally on pre-M1 src and by one mutant per task, plus the A fixes' red/green); F2 MINOR hand-written fixtures (T018): accepted, already a Follow-up (probe before release). Verify green. Scoped gate at a3e43dbac: fmt + clippy (core, workspace, `-D warnings`) green; Verify suites green (github_filter 47, github_gh_cli 17, jira_query, github_privacy, github_search, github_load); `cargo test --workspace --no-fail-fast` red only in 6 permission tests that cannot fail a write as root (container runs as uid 0: settings_refuses_save_over_failed_read, settings_write_is_logged, worktree_leftovers ×2, daemon sessions worktree_delete_blocked…, daemon settings …unreadable…), untouched by M1 |
| M1 Review A + B | 2 | 696c1b64d4d29fbb930a0f8a14f7a00d55e8c9c5:e46e905c271ce9ab7b57e8b19ce28bd9e9b0c8ea | CLEAN (scoped, sonnet; A's three fixes and B F1 confirmed, github_filter 48, github_gh_cli 18). B scoped (MAJOR fix) + A scoped on the fix diff. Round 1's snapshot is stale after the rebase onto origin/main (`review-snapshot.sh diff` exits 2); M1's diff is unchanged by it (same patch-id, a3e43dba→04e671637, d414072c→15174b40f), so the fix diff is `git diff 15174b40f e46e905c2`. Scoped gate: fmt, clippy core `-D warnings` green; github_filter 48, github_gh_cli 18, jira_query 36, github_privacy/search/load green |

## Notes

- M1: `UserQuery::condition()` collapses whitespace runs only outside quoted strings (data-model §3 says "spaces collapsed"; inside a quoted phrase the spacing is the user's search text). A `sort:` token ends before whitespace, `(`, `)` or `"`, so cutting it never removes a parenthesis (research R5 leaves the token end open). `ScopedSearch::listing` is built for any filter; the default basic filter is kept off it by `GithubFilter::is_default` at the caller (search-rules §4 "not constructible"). `FilterFailure`'s `Debug` counts GitHub's messages instead of printing them (they can quote the query, FR-024).
- Tasks unit: M1 is Foundational + US1's core only (no UI), as 1186's M1: Setup + Foundational + all of US1 is ~30 tasks and over 800 lines (milestones.md rule 3); split at the core/client seam. US2 split along its scenarios: basic (M3) and advanced (M6). US3 and US4 share M5 (a Run without its refusal display would be half-wired, rule 6). The plan's 4-milestone delivery order is superseded by tasks.md §Milestones, as the plan says.
- `speckit-analyze` is a forked skill not loadable in this cloud unit; checked inline instead: FR-001–FR-027, SC-001–SC-007 and quickstart §B B1–B18 each map to a task and a milestone; no CRITICAL or HIGH found.
- checklists/requirements.md: ticked "No [NEEDS CLARIFICATION] markers remain" (spec has none; D1 is a recorded provisional clarification, tracked in Open escalation). For the tasks reviewer to confirm.

- M2: **Deviation from selector-ui §1 (layout)**: the GitHub **Filters** button trails the row heading the picker (with **Clear filters** before it while a filter is chosen), not the search's row. Beside the search it narrowed the search's floating list by 168 px, and long issue titles then wrapped into rows the eight-row list could not show whole: `issue_rows_show_all_text` (×2) and `picker_row_tooltip_clears_its_row` failed. That row is where M5's Basic/Advanced switch leads (selector-ui §1), so M5 adds no row for it. Cost: the GitHub issue form is one row taller than 034's with the default filter (SC-007 "looks as before" holds for the list, the statement and what is sent, not the height; any Filters control changes the look). `github_filters_fit` asserts the placement taken; `layout_snapshot.txt` re-recorded (only the 9 GitHub issue states changed, plus the 2 new). For review B / the visual pass to weigh.
- M2: **Spec tension, Edge "Same label" vs quickstart B4**: T021/selector-ui §2 offer the labels "minus labels already chosen in either list", so `blocked` excluded is not offered in Include; B4's "try to include `blocked` → it moves" is reached by removing the chip, then picking it in Include. `BasicFilters::include_label` still moves a label (core, M1). For review B.
- M2: the empty filtered list shows "No issue matches these filters." with **Clear filters** in the row above (not a second button beside the text), as 1185 does for Jira. `GithubSelector::from_remembered` returns the default until M3 (T034). A rejected search *beyond* the list (rare: the basic filter was accepted for the listing) shows in 034's search-failure line ("GitHub did not accept the search query. <messages>") with Retry; only a rejected listing takes the list's place (FR-018).
- M2: the reducer tests stand in for the shell (`serve` runs `load_filtered` / `load_label_choices` / `search_filtered` on a `FakeIssueSource` exactly as `shell::github_filter` does) and assert `asked_searches()`; the shell itself is covered by the named-events scan. `on_filter` (shell) re-runs a pending search after a filtered reload with text typed (FR-016).

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, category 1 (product or scope decision the repo does not settle). On the user's decision card; proceeding on the recommended option as D1 (provisional). Move to Decisions as decided by user when answered.

- **Q1**: Issue #1400 lists "optionally assignee, author and milestone" among the basic filters. Are those three basic filters part of this feature?
  - **(Recommended) Include them, as P3 in the last milestone** — spec.md US6/FR-003/FR-005 already specify them in full; the issue names them in its basic-filter proposal; shipping them last keeps the P1 use case (exclude `in-progress`/`blocked`) unblocked and lets them be dropped late if needed.
  - **Leave them out; advanced mode covers them** (`no:assignee`, `author:@me`, `milestone:"v2.4"`) — smaller scope: no assignable-user search, viewer lookup or milestone choice loads; US6 moves to Out of Scope for a later issue. Precedent: 1185 left assignee out of the Jira basic filters (specs/1185-jira-selector-filters/spec.md Out of Scope).
  - Evidence checked: issue #1400 body (only "optionally"), issue comments (none), specs/1185-jira-selector-filters, specs/1186-jira-advanced-jql, specs/034-github-issue-worktree — none settles it.

## Follow-ups not done

- M1 (T018, deviation): `ISSUE_ADVANCED` (R3) and a refused search's error shape (R7, incl. `\` inside a quoted term) **not probed** live: no signed-in `gh` in the cloud container. `parse_search_page` tests use hand-written answers (search-rules §6), and `tests/fixtures/gh/search_filtered_page.json`, `search_rejected.json`, `labels_page.json` are hand-written, not recorded. Probe with a signed-in `gh` before release; if GitHub refuses the type, only `github_filter::search_type!` changes.
- M2 unit: `mise` is not installed in this container; the gate steps ran as `scripts/build-lock.sh cargo …` (the M2 scoped gate: fmt --all --check, clippy -p micold-client --all-targets -D warnings, test -p micold-client). LSP not tried (grep used). Quickstart §B with `fake_gh` not run (no Agent tool for the visual pass in this unit).
- M1 unit: LSP tool not available here either (ToolSearch found no `LSP`); symbols checked with grep.

- Plan unit: LSP tool not available in this session (ToolSearch found no `LSP`); symbols checked with grep. ISSUE_ADVANCED and the refused-search error shape not probed live (no signed-in `gh`): research R3/R7, M1 fixture probe.
- Cloud session: `gh` unauthenticated, so `branch-start.sh` and `issue.sh` were skipped (orchestrator set labels/assignee via GitHub MCP; branch already at origin/main 828bc8e24). Not pushed: the orchestrator pushes with the design PR.
