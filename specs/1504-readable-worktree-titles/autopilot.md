# Autopilot ledger — #1504 readable-worktree-titles

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1504 (issue #1504: Worktrees without a GitHub issue title should still show a readable overview); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1504
- **Worktree branch**: claude/project-thread-t2a600
- **Started**: 2026-10-27
- **Phase**: M3 implemented (T025–T032) on branch 1504-m3 (worktree /home/user/wt-1504-m3), stacked on M2's head 075ac3985 (M2 PR not opened yet); full gate (`scripts/gate.sh`) green at 53ba8af40; not pushed; review A and review B and the visual pass pending
- **Next step**: orchestrator runs M3 review A (code-review high) and review B (conformance, sonnet) on `075ac3985..HEAD` of 1504-m3 and the visual pass (quickstart §B rows A–E, G, I with source words; Refresh: close E's PR, archive G's session); M2's review B round 2 and PR still pending on 1504-m2; M3 PR text in scratchpad pr-title-m3.md / pr-body-m3.md

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1557 | design | merged | 24a2a839f |
| #1566 | M1 | merged | 2104379be |
| — | M2 | pending (not opened; branch 1504-m2 at 075ac3985) | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016, T048 | full | Chooser behind card/row/header; GitHub ticket from the branch titles the worktree | #1566 | merged |
| M2 | T017–T024, T049 | full | Jira title ticket shows its summary; branch-only key gets no chip | — | implemented, reviews pending |
| M3 | T025–T032 | full | PR and main-session titles; source named in row/header/expanded-card tooltip | — | implemented on 1504-m3, reviews pending |
| M4 | T033–T041 | full | Spec heading and own-commit titles, read off-thread on list change / Overview open | — | pending |
| M5 | T042–T046 | light | Failed GitHub title reads retried after 5 min; NoIssue never re-asked | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Spec review F1: widen "linked ticket" to branch names? | No: new term "title ticket" for title lookup only; sidebar tag, Jira chip and Jira status keys unchanged | orchestrator | spec review round 1 F1 |
| D2 | clarify | Session title above PR/spec titles in FR-001? | No, below (as drafted) | agent | specs/813-worktree-overview/spec.md#FR-015; specs/032-untitled-session-labels/spec.md#Clarifications |
| D3 | plan | Branch tickets like `feat/1504-readable-titles` (no `_` boundary)? | Also read the number via `short_branch`'s rule; spec Assumption's `-` form is not what `naming` parses | agent | research.md R2; crates/micold-core/src/naming.rs split_name; overview_text.rs short_branch |
| D4 | plan | Card title has no tooltip since 1121: where does FR-008's source go? | Row tooltip, header tooltip, expanded card line | agent | research.md R6; docs/user-guide/overview.md L131 |
| D5 | plan | Plan review 1 F3: keep Jira summaries on a failed refresh, given FR-004 "a failed or absent reading gives no title"? | Yes: FR-004 read as "a failure never yields a title of its own"; a summary from an earlier successful read stays until the board retires it, like the status | agent | research.md R4; jira_status FAILURES_BEFORE_RETIRING |
| D6 | plan | Plan review 1 F1: when are spec/commit facts read? | On worktree list or active-project change (main.rs fingerprint check, covers set_worktrees, created, included, excluded) and on Overview opening; FR-006 "once per worktree list refresh" read as once per title-refreshing occasion | agent | research.md R5; main.rs:490; features/worktree.rs:266,339,348 |
| D7 | tasks | M1 = Foundational (8 tasks) + US1 GitHub branch ticket (8) = 16 tasks, over ~10: split further? | No: US1 split along scenarios (M1: 1,3; M2: 2,4) and US2 (M3: 1,3,5–7; M4: 2,4); Foundational alone is never a milestone and is a behaviour-preserving refactor, so M1's new behaviour is 8 tasks | agent | milestones.md rules 1, 3 |
| D9 | tasks | Analyze F1: spec "git missing" (commit only) vs plan "no local git → no read" | Two cases: `git` failing/missing loses only the commit (spec still read); no local git capability (remote daemon, 027) reads neither, paths are the daemon's. Spec edge case split, L7, T034/T036/T040 | agent | research.md R5; shell/capabilities.rs `git`/`without_local_git` |
| D10 | tasks | Analyze F4: 813/1120 amendment notes only at close? | No: T048 moved into M1, with the first title-order change | agent | spec FR-013 |
| D11 | M1 | T005/T012 put card-vs-row agreement in `tests/features_sidebar.rs`, whose header forbids building `State` or naming another feature | The State-based agreement tests (branch ticket, folder wins, rename, Default, orphan) live in `tests/overview.rs`; `features_sidebar.rs` pins `row_title` over each `LocationTitle` source and the rename | agent | tests/features_sidebar.rs module doc (feature 021, SC-004) |
| D8 | tasks | Optional after_tasks hooks (tdd plan, docguard score), optional before_tasks docguard review | Skipped (optional; tests already first in every phase) | agent | .specify/extensions.yml |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 541682421a341a1fafc3450b6b9036cdf9e549c0:828bc8e24442d9c9685869c35fe69134934bd233 | CHANGES: 3 MAJOR, 2 MINOR — all fixed (F1 title ticket term, orchestrator choice; F2 FR-013 amendment; F3 edge cases; F4 FR-003; F5 SC-005) |
| Spec review | 2 | f2abbf7224da5e36c1fdee21ec8fe71ba20af4ab:45a9b105ae6d57805c765b6343f57f6e573a90c7 | CLEAN (3 MINOR folded into clarify: FR-002 wording, FR-003 rewrap, SC-005 measurable) |
| Plan review | 1 | a6363620134ea5fad5070c1cb4dacc71154cfb1e:0716b1a0459687647d249c43c1c5168d5a7f7e19 | CHANGES: 1 MAJOR, 3 MINOR — all fixed (F1 facts read on list/project change + Overview opening, `facts_round`/`facts_asked` replace `listing`, main_tests for created/included; F2 alternatives in R7/R9/R10/R11; F3 interpretation D5; F4 spec read by GitHub title ticket, D2 callers corrected) |
| Plan review | 2 | 4d7a2d63c9bec458651d6712ce7f7909685829a8:157212aa9334caef6d3266340e08ae115b0f6f8f | CLEAN (1 MINOR fixed, prose only: FR-006 now reads facts on each worktree list change and each Overview opening, spec Amendments note) |
| Tasks review | 1 | ce45dfee3920e1244eda9ffc375b938c4debd12a:8e83f798b46c61f3251596ce88fb9ca69c3f94e1 | CHANGES: 3 MAJOR, 2 MINOR — all fixed (F1 Verify commands: one filter per `cargo test --lib`, `mise run test-core` alt, `--bin micold-ai-ide` with named test prefixes `worktrees_fingerprint`/`title_facts`/`title_answer`; F2 M1/M2 Verify title text only, source words in M3; F3 [P] dropped from T003/T004, parallel line fixed; F4 M3 Refresh uses E (close PR) and G; F5 stamp with `micold_core::unix_time::unix_secs()`). speckit-analyze (0 CRIT/HIGH, 4 MED, 8 LOW): MED fixed — F1 git-missing split (D9), I1 `numeric_ticket` removal moved to T015, C1 T018 Jira-off case, C2 T005 Default/orphan case; LOW fixed — F2 lowercase key edge case + L2 folder-key-vs-branch-number row, F3 FR-008 names the three surfaces, F4 T048 in M1 (D10), F5 neutral clock, C4 non-ASCII path in T033, C6 stale edge case narrowed to spec/commit; LOW not fixed — C3 timeout test, C5/C7 per-occasion retry tests (covered by L5 table tests in T042) |
| Tasks review | 2 | 98e1d4227be086796612c25694acc9ade9a02eb6:b89c96749619ccbd77e44926bd9633d14d02213e | CHANGES: 1 MAJOR, 1 MINOR — all fixed (F1 T014 now also moves `request_titles` (issues.rs ~785–810) and the existing tests/overview.rs ~1352 call, and removes `numeric_ticket`, so it builds alone; T015 keeps only the fingerprint; F2 [P] dropped from T002) |
| Tasks review | 3 | 05ac0485267c46324864564487a846e51b5da779:5cb42ed07b4d2000c3169efa63d0e0228df28d38 | CLEAN (cosmetic note: T014 could cite both calls at tests/overview.rs:1352–1353; not needed) |
| M1 review A (code-review high) | 1 | 832dfdb47fcacca1e78bd06d65e1d4b9fdf9af48:7fbb9ec20822de72e5b0e383eeb033f2b096b67f | CHANGES: 1 MAJOR, 2 MINOR — all fixed (F1 a folder ticket of neither kind no longer stops the branch being read; F2 a non-conventional namespace segment (`jh/`, `user/`) is skipped before the branch ticket is read; F3 Jira key rule reused from `issue_link::ticket_ref` (digits after the first letter); L2 rows and tests added) |
| M1 review B (conformance, sonnet) | 1 | 832dfdb47fcacca1e78bd06d65e1d4b9fdf9af48:7fbb9ec20822de72e5b0e383eeb033f2b096b67f | CLEAN (1 MINOR: no TDD cycle log; red runs named in the PR body) |
| M1 visual pass | 1 | 7fbb9ec20 | PASS: quickstart §B rows A, B and rename, light and dark (/mnt/project-files/issue-1504/) |
| M1 review A (code-review, scoped) | 2 | d074ecd6fb6b943300885d215029a356dea395db:104b7ed0222554ea88387a5c7dc81eec981d4073 | CLEAN (F1–F3 fixed; `issue_ref` unchanged for callers); full gate green at 104b7ed02 |
| M2 review A (code-review high) | 1 | 9450b457e924177b36a486a1b443e093f84ee3b1:f99f06acf669b05a4624510539054cffd45f92e7 | CLEAN, 2 MINOR — both fixed, no round 2 (only MINORs): (a) `without_a_jira_summary_the_title_falls_through_to_the_branch` now asserts the summary after the t=1 read and after the first failure; (b) `StatusBoard::apply` doc names the one-read-in-flight assumption `due` ensures |
| M2 review B (conformance, sonnet) | 1 | 9450b457e924177b36a486a1b443e093f84ee3b1:f99f06acf669b05a4624510539054cffd45f92e7 | CHANGES: 1 MAJOR, 1 MINOR — F1 fixed (T049: `DATA_USE`, both selector notices and settings.md now disclose the title and keys named only in a branch; tests changed first, red); F2 no TDD cycle log — accepted as in M1, red runs in ledger and PR body |
| M2 visual pass | 1 | f99f06acf | PASS: quickstart §B rows C, D; stub request asks status+summary incl. ABC-43 (/mnt/project-files/issue-1504/m2-overview-sidebar.png, m2-jira-stub-request.log) |
| M3 review A (code-review high) | 1 | bf46fa9238873d669c14b76be2cab7ff567a5c37:53ba8af404b6ea5388c08d7cdcd5d6c142d710bc | pending (orchestrator; `075ac3985..HEAD`) |
| M3 review B (conformance, sonnet) | 1 | bf46fa9238873d669c14b76be2cab7ff567a5c37:53ba8af404b6ea5388c08d7cdcd5d6c142d710bc | pending (orchestrator; `rubrics/conformance.md`, re-run Verify) |
| M3 visual pass | 1 | 53ba8af40 | pending (quickstart §B rows A–E, G, I + Refresh E/G; showcase *title sources, expanded*) |

## Red runs (M3)

Each new test was written before the code and run red against a stub (`TitleSource::words`
returning `None`, `with_title_line` taking the `LocationTitle` but adding no source line,
`ExpandedText.source` always `None`, `location_title` still ignoring the pull request and session),
so every red is a failing assertion, not a compile error:

- `cargo test -p micold-core --lib -- location_title context_header overview_text` (T025, T027):
  exit 101, 3 failed — `each_source_has_its_l3_words_and_the_folder_none` left `None`, right
  `Some("From GitHub issue #1504")`; `the_title_tooltip_names_the_titles_source` left
  `"Add readable titles\nx"`, right `"Add readable titles\nFrom pull request #1520\nx"`;
  `the_expanded_text_names_the_titles_source` left `None`, right `Some("From pull request #1520")`.
- `cargo test -p micold-client --test features_overview` (T026): exit 101, 4 failed —
  `an_open_or_draft_pull_request_of_the_branch_titles_the_card` left
  `("claude/project-thread-7041o6", Branch)`, right `("Add readable titles", PullRequest(1520))`;
  `the_main_sessions_title_titles_the_card` left `(…, Branch)`, right
  `("Fix the flaky resize test", Session)`; `the_pull_request_wins_over_the_session` and
  `a_reading_or_an_archive_changes_the_title_with_no_other_message` left `Branch`.
  `nothing_gives_the_branch_then_the_folder` passed (today's behaviour, kept).
- `cargo test -p micold-client --test features_sidebar two_line` (T027): exit 101, 2 failed —
  `the_tooltip_begins_with_the_full_title_and_names_the_full_branch` left `Some("Name: ci-red")`,
  right `Some("From GitHub issue #1117")`; `the_tooltip_names_the_titles_source_after_the_title`
  left the tooltip without the `From pull request #1520` line.

Green after T028–T030: the same commands, all passed (core 40, features_overview 13).

## Red runs (M2)

Each new test was written before the code and run red; the red was a compile failure in every case,
since the tests name the new `KeyedIssue`, `StatusBoard::begin(status, summary)`, `summary_of` and
`branch_title_keys`:

- `cargo test -p micold-core --lib jira_status` (T017): exit 101, 35 errors — 11× E0061 `begin`
  takes 1 argument but 2 were supplied, 11× E0599 no method `summary_of` on `StatusBoard`, 5× E0422/
  E0425 no `KeyedIssue`, 8× E0609 no field `key`/`status`/`summary` on the `(String,
  Option<TicketStatus>)` tuple.
- `cargo test -p micold-core --test jira_parse` (T017): exit 101, E0609 no field `key` / `summary`
  on `(String, Option<TicketStatus>)` at `tests/jira_parse.rs:174`.
- `cargo test -p micold-client --test features_jira_status --test jira_status_sidebar --test
  jira_http_adapter --test overview --no-run` (T018–T020): exit 101 — E0432 unresolved import
  `micold_core::jira_status::KeyedIssue`, E0061 `begin` arity (`tests/overview.rs:1502`), E0609 no
  field `key`/`status` (`tests/jira_http_adapter.rs:349,350,375,603`). The T019 `fields` assertion
  and the T018 `main_tests` gate test could not reach an assertion red behind the type change.

Green: core lib `jira_status` 26 passed, `jira_parse` 12 passed; client `features_jira_status` 6,
`jira_status_sidebar` 13, `jira_http_adapter` 35, `overview` 94, `features_sidebar` 105,
`jira_status_overview` 5 passed.

Placement notes: T020's card-and-row tests live in `tests/overview.rs` (beside M1's `card_and_row`
helper, which builds the card and the row from one state) rather than `tests/features_sidebar.rs`;
T018's "switch off or no connection sends nothing" case lives in `main_tests.rs`
`jira_status_driver` (the gate is the shell's, `State::plan` never sees the switch).

T049 red (disclosure, review B F1): `cargo test --no-fail-fast -p micold-client --test
jira_query_state --test jira_source_state --test jira_keep_or_remove_is_drawn --test overview`
exit 101 — `the_page_discloses_the_background_status_read` ("status and title" missing from the
note), `the_notice_in_advanced_mode_names_the_query` and
`a_project_without_a_github_remote_lists_jira_and_offers_no_source_choice` (exact text differs);
`overview` 94 passed (review A (a) adds asserts on existing behaviour, so no red).
The longer selector notice wraps one more line in the Jira New-worktree dialog: the full gate's
`layout_snapshot` failed on the `add-worktree-dialog-jira-*` states only (dialog 16 px taller,
`0/8/…` paths), accepted with `UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test
layout_snapshot`.

Pending (M2): scoped review B round 2 (sonnet, `rubrics/conformance.md`, re-run Verify) on the
T049 fix commit `1b94d0344..ef2d329bc` (the round-1 snapshot is stale after the rebase onto
2104379be, so the diff is given as a commit range). Full gate green at ef2d329bc.

Round-1 reviews (M2, done): review A (`code-review` high) and review B (`rubrics/conformance.md`,
sonnet) on `ac5ada9ea...HEAD` (`code-review` high) and review B (`rubrics/conformance.md`,
sonnet) on `ac5ada9ea...HEAD`, brief `brief.py milestone specs/1504-readable-worktree-titles M2`;
visual pass on quickstart §B rows C–D (title text only) with a loopback Jira stub. Round-1 snapshot
for A and B: `9450b457e924177b36a486a1b443e093f84ee3b1:f99f06acf669b05a4624510539054cffd45f92e7`.

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
