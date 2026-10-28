# Autopilot ledger — #1504 readable-worktree-titles

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1504 (issue #1504: Worktrees without a GitHub issue title should still show a readable overview); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1504
- **Worktree branch**: claude/project-thread-t2a600
- **Started**: 2026-10-27
- **Phase**: done — close PR ready on branch 1504-close (rebased onto origin/main 4317a0a3a); not pushed, title/body in scratchpad pr-title-close.md / pr-body-close.md
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1557 | design | merged | 24a2a839f |
| #1566 | M1 | merged | 2104379be |
| #1568 | M2 | merged | fb29142a7 |
| — | M3 | #1572, merged | 6f67d1beb |
| — | M4 | #1575, merged | 0045f46e6 |
| — | M5 | #1580, merged | 4317a0a3a |
| — | close | ready (branch 1504-close; the orchestrator pushes and opens it; "Fixes #1504") | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016, T048 | full | Chooser behind card/row/header; GitHub ticket from the branch titles the worktree | #1566 | merged |
| M2 | T017–T024, T049 | full | Jira title ticket shows its summary; branch-only key gets no chip | #1568 | merged |
| M3 | T025–T032 | full | PR and main-session titles; source named in row/header/expanded-card tooltip | — | implemented on 1504-m3, reviews pending |
| M4 | T033–T041 | full | Spec heading and own-commit titles, read off-thread on list change / Overview open | — | implemented on 1504-m4, gate and reviews pending |
| M5 | T042–T046 | light | Failed GitHub title reads retried after 5 min; NoIssue never re-asked | — | implemented on 1504-m5 (stacked on M4 05897a571), gate and reviews pending |

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
| M3 review A (code-review high) | 1 | bf46fa9238873d669c14b76be2cab7ff567a5c37:53ba8af404b6ea5388c08d7cdcd5d6c142d710bc | CLEAN, 2 MINOR — both fixed: (a) a renamed row's tooltip led with the location title the row does not show (1125 FR-012): `with_title_line` takes the rename and leads with it, no source line (test `a_renamed_rows_tooltip_leads_with_the_rename_not_the_pull_request_title`); (b) = review B F1 |
| M3 review B (conformance, sonnet) | 1 | bf46fa9238873d669c14b76be2cab7ff567a5c37:53ba8af404b6ea5388c08d7cdcd5d6c142d710bc | CHANGES: 1 MAJOR, 1 MINOR. F1 [MAJOR] fixed: `with_title_line` skipped the source line for a Branch title (L3 exempts only Folder); now only Folder names no source, and a title that repeats the Name/Branch line is dropped but its words stay (`Branch name\n{tooltip}`); test rewritten, worktrees-and-sessions.md aligned with overview.md. F2 [MINOR] deferred to M4 (Follow-ups) |
| M3 review B (conformance, sonnet) | 2 | 0348aec6b895fb6c0f147c90cfb52a4d3a7b2de6:e51e05b76a1db8c6e2f7a163b09a60dbd30c2dc7 | pending (orchestrator, sonnet; fix diff `review-snapshot.sh diff bf46fa9238873d669c14b76be2cab7ff567a5c37:53ba8af404b6ea5388c08d7cdcd5d6c142d710bc` = `253f3f912..e51e05b76`; re-run Verify) |
| M3 visual pass | 1 | e51e05b76 | pending (title, row tooltip, header tooltip, expanded card for rows A, C, E, G, I, a folder-titled and a renamed worktree; Refresh E/G) |
| M4 review A (code-review high) | 1 | 87aa43d50849718e589e6ac1d6cd152c35b98131:72832ebf85d79eeadb243dfdbe89ea30f2859b3c | CLEAN, 3 MINOR (real defects, all fixed test-first): F1 own commit could be a merge from main after syncing → `log -1 --no-merges` (git.rs), test `a_merge_from_the_default_branch_is_not_the_own_commit_subject`, contract L7 + data-model + research updated; F2 `ProjectSwitched` reset `facts_round`, so A→B→A let a stale round-1 answer through → round never reset, test `a_late_answer_from_an_earlier_visit_to_the_project_is_dropped`, data-model updated; F3 facts trigger missed invalid→valid / moved worktree → `location_titles::facts_fingerprint` (issue fingerprint + status + path; `WorktreeStatus: Hash`) drives the facts read in main.rs, test `title_facts_a_worktree_turning_valid_or_moving_is_asked_again` |
| M4 review A (scoped re-check, sonnet) | 2 | — | pending (orchestrator; fix diff `6abaeabc2..HEAD`; MINOR fixes, does not count toward the limit) |
| M4 review B (conformance, sonnet) | 1 | 87aa43d50849718e589e6ac1d6cd152c35b98131:72832ebf85d79eeadb243dfdbe89ea30f2859b3c | CLEAN, 1 MINOR not changed (facts read twice when one message both changes the list and opens the Overview: harmless, the later round wins) |
| M4 visual pass | 1 | 72832ebf8 | pending (quickstart §B rows F and H: card title, row first line + tooltip, header tooltip, expanded card source line; Refresh: open a PR for H → PR title; late commit in a new worktree → shown after reopening the Overview; showcase *title sources, expanded* with the Spec and Commit cards) |
| Close: tdd-verify (autopilot-worker) | 1 | b240c4c70 (`tdd/verification.md`) | FAIL on process evidence only (M1 and fix-test reds missing; no unbuilt behaviour; 3 of 3 mutants caught). Remediated with after-the-fact mutation red runs for M1 T002–T005, T009–T012, F1–F3 and M5 A1–A4, B2: 22 of 24 mutants caught at first; the 2 survivors (empty branch accepted by `choose`; `worktree_tickets` passing no branch) were killed by new tests `an_empty_branch_falls_through_to_the_folder` (red: `left: (Branch, "")`) and `title_answer_a_ticket_only_in_the_branch_is_asked_for` (red: `left: []`), each green on the real code; the restoring edits changed no code |
| Close diff review (sonnet) | 1 | `git diff 1e4021a18..HEAD` | CHANGES (2 MAJOR, 2 MINOR), all fixed: spec Status names the close PR; quickstart Part A gate claim has the row below; visual passes recorded (M3 PASS 7 of 7, M4 PASS, M5 none needed: no UI changed) and T050 ticked; M4/M5 merge SHAs filled |
| M3 review B (conformance, sonnet) | 2 | 0348aec6b895fb6c0f147c90cfb52a4d3a7b2de6:e51e05b76a1db8c6e2f7a163b09a60dbd30c2dc7 | CLEAN (1 MINOR, no change) |
| M5 round 2 (combined A+B, scoped, sonnet) | 2 | — | CLEAN, 1 MINOR fixed (107-column comment in `request_titles`) |
| Close: speckit-converge | — | — | skipped: with the two rows above every milestone has a CLEAN review B (M1 r1, M2 r1 fixed then accepted, M3 r2, M4 r1, M5 r2) |
| Close gate | — | close HEAD with the two new tests | full `scripts/gate.sh` exit 0 (fmt, clippy, workspace tests, script tests) on the rebased tree including the two test-strength tests |

## Red runs (M5)

Reducer (`titles_to_fetch`, `TitleLookup`) was written with the shell mapping stubbed to
`Failed { at: 0 }`; every red is a failing assertion:

- `cargo test -p micold-client --test overview a_retried_title_is_pending_and_the_card_never_shows_loading`
  (T042): exit 101, `left: Failed(10)  right: Pending` (a retried title stayed `Failed`; fixed by
  making `TitlesRequested` reset `Failed` to `Pending`).
- `cargo test -p micold-client --bin micold-ai-ide title_answer` (T043): 5 failed —
  `..._no_github_remote_is_no_issue` and `..._number_that_is_no_issue_is_no_issue` with
  `left: Failed(0)  right: NoIssue`; `..._lost_remotes_read_is_failed`,
  `..._failed_read_is_failed_with_the_time` and `..._no_gh_is_failed` failing "stamped 0, not now".
- Green after T044/T045: overview 96 passed, `title` bin tests 21 passed.
- Full gate (`scripts/gate.sh`) green on the third rerun: the first two runs hit flakes unrelated to
  this diff while the M4 gate ran concurrently on the shared target (`capture-harness` private X
  display; `guide-own-image` libpq check; then daemon `structured_permission`
  `a_pending_request_marks_its_session_waiting...` Unknown vs AwaitingInput), each passing alone.
- Review round 1 fix (B1), red run taken after the fact by stubbing `titles_to_fetch` so a `Failed`
  entry is never due: `cargo test -p micold-client --test overview titles_to_fetch` exit 101,
  `titles_to_fetch_re_asks_a_failed_read_only_after_the_retry_interval` failed at "due at exactly
  300 s" (`left: []  right: [1]`); stub then restored. The other T042 test
  (`a_retried_title_is_pending_...`) and the five `title_answer_*` tests have the genuine reds
  above. The round 1 fix tests (A1 switch-off drops pending, A2 clock back, A3 seed from Failed,
  A4 outage short-circuit, B2 NotOpenIssue/overflow) were written together with the fix, with no
  separate red run; stated plainly. The feature records no tdd/cycle-log file.
- Full gate green after the round 1 fixes (GATE_EXIT=0).
- Round 2 (combined A+B, scoped, sonnet): CLEAN, 1 MINOR (a 107-column comment in `request_titles`),
  fixed. M4 merged as 0045f46e6 (#1575); M5's five commits plus the rewrap were rebased with
  `git rebase --onto origin/main 05897a571` (the merged M4 commits dropped, no conflicts) and the full
  gate (`scripts/gate.sh`) is green on the rebased head (GATE_EXIT=0).
- Round 1: review A CHANGES (7 MINOR, all fixed: A1 switch-off removes Pending, A2 `now < at`
  due, A3 seed Failed from the form, A4 outage stops the batch, A5 rig helper, A6 comments, A7 docs);
  review B CHANGES (1 MAJOR, 3 MINOR, all fixed). Round 2 pending (scoped, sonnet).
- Reviews pending (round 1, superseded): review A (`code-review` high) and review B (conformance rubric
  `.claude/skills/speckit-autopilot/rubrics/conformance.md`) on `05897a571..HEAD`; no visual pass
  (no UI change: only which titles are asked and when).

## Red runs (M4)

Each new test was written before the code and run red against stubs (`spec_title` /
`read_spec_title` / `GitCli::own_commit_subject` returning `None`, `FakeGit::with_own_commit`
recording nothing, the `TitleFactsRequested`/`TitleFactsRead` arms doing nothing, `location_title`
not reading facts, `read_facts` returning `Task::none()`), so every red is a failing assertion:

- `cargo test -p micold-core --lib location_title` (T033): exit 101, 4 failed —
  `a_spec_heading_loses_its_feature_specification_prefix`,
  `the_spec_title_is_read_from_the_folder_feature_json_names`,
  `without_feature_json_the_spec_folder_of_the_ticket_is_read`,
  `a_worktree_with_a_non_ascii_path_is_read` (each left `None`, right `Some(..)`); the none-cases
  pass against the stub, as expected.
- `cargo test -p micold-core --test git_fake own_commit` (T034): exit 101,
  `own_commit_subject_answers_what_was_scripted_for_the_dir` failed (left `None`).
- `cargo test -p micold-core --test review_git own_commit` (T034): exit 101,
  `the_own_commit_subject_is_the_branchs_latest_commit_the_default_branch_lacks` left `None`, right
  `Some("fix(overview): wrap long titles")`; the no-base / not-a-repo test passes against the stub.
- `cargo test -p micold-client --test features_overview title_facts` (T035): exit 101, 4 failed —
  `a_request_bumps_the_round_and_records_it_per_dir` left `0`, right `1`;
  `only_the_last_rounds_answer_for_a_listed_dir_is_stored` and
  `an_answer_from_another_project_is_dropped` left `None`, right `Some(TitleFacts { .. })`;
  `stored_facts_title_the_card_in_order` left `("Fix the flaky resize test", Session)`, right
  `("Overview cards, readable layout and content", Spec)`. Project-switch and Debug pass on the stub.
- `cargo test -p micold-client --bin micold-ai-ide title_facts` (T036): exit 101, 4 failed —
  listing / created+included / Overview opening each left `[]`, right the dirs; the end-to-end
  `title_facts_a_listed_worktrees_spec_heading_is_read_and_stored` left `None`, right
  `Some("Readable titles")`. `title_facts_without_local_git_nothing_is_asked` passes on the stub.
- Review A fixes (written before each fix, run red on the unfixed code):
  F1 `cargo test -p micold-core --test review_git own_commit`: exit 101,
  `a_merge_from_the_default_branch_is_not_the_own_commit_subject` left
  `Some("Merge branch 'main' into feature")`, right `Some("feature work")`.
  F2 `cargo test -p micold-client --test features_overview title_facts`: exit 101,
  `a_project_switch_clears_the_facts` (now expects the round kept) left `0`, right `1`;
  `a_late_answer_from_an_earlier_visit_to_the_project_is_dropped` failed (stale answer stored).
  F3 `cargo test -p micold-client --bin micold-ai-ide title_facts`: exit 101,
  `title_facts_a_worktree_turning_valid_or_moving_is_asked_again` "turned valid" left `1`, right `2`.

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

Review fixes (A minor (a), B F1), `cargo test -p micold-client --test features_sidebar -- tooltip`
with `with_title_line` given an ignored `renamed` parameter: 2 failed as assertions —
`a_renamed_rows_tooltip_leads_with_the_rename_not_the_pull_request_title` (left opened with
`Add readable titles\nFrom pull request #1520`, right `my pet name`) and
`the_tooltip_names_the_titles_source_after_the_title` (Branch title: left today's tooltip, right
`Branch name\n…`). Green after the fix: features_sidebar 107 passed.

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

- ~~M4: add `TitleSource::Spec` and `TitleSource::Commit` to `showcase::samples::title_sources()` (M3 review B F2).~~ Done in M4.
