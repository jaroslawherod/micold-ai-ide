# Cycle Log: Pull Request and Check Status for Each Worktree

Append only. Newest last. Every entry's `red` block is the evidence that the test
existed and failed before the implementation.

## Baseline

- suite (fast subset): `mise run test-core` (`scripts/build-lock.sh cargo test -p micold-core --all-targets`)
  -> 1510 passed, 0 failed, 7 ignored (135 test-result lines), exit 0
- suite (workspace): not re-run at planning time (slow, shared build lock). Baseline is CI's green
  `main` at `3d52e83e` (merge-base of this branch; `ci.yml` conclusion `success`); the branch adds
  only `specs/040-worktree-pr-ci-status/` documents on top of it.
- commit: `9501106d`
- recorded: cycle 0, before any change

## Notes and deviations (M1)

- **Batched reds, per test file.** As in features 033 and 034: every test of one file is written
  first, run against the code as it stands (or a stub that only makes the symbols resolve), and each
  is observed failing on its own assertion. A test that passed at that point is held to the
  deliberate-mutant check after green. The inner loop runs the one test target; the per-cycle suite
  is `mise run test-core` (the core crate, where every M1 behaviour lives, and the suite the baseline
  above was taken with); the full workspace suite runs in `mise run gate` before the PR.
- **Where the answers were recorded.** The project's repository has no draft, no review decision,
  no failing or running check on an open pull request, and no fork's pull request, so those answers
  were read from public repositories (`cli/cli`, `microsoft/vscode`) with read-only queries.
  `crates/micold-core/tests/fixtures/gh/pr_README.md` names the repository and the branches of each
  file. The three request-limit answers are written from GitHub's documentation, as tasks.md T001
  allows.
- **`.gitattributes`.** `crates/micold-core/tests/fixtures/gh/pr_*.txt -text` was added so the
  recorded `\r\n` header line ends survive `* text=auto eol=lf`.

## Cycle 1: U1 — the recorded answers are present and hold no credential

- test: `crates/micold-core/tests/pull_request_parse.rs::every_recorded_answer_is_present_and_holds_no_credential` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test pull_request_parse`
  -> `pr_three_branches.txt is one of the 14 recorded answers: No such file or directory (os error 2)`
  (`test result: FAILED. 0 passed; 1 failed`)
- green: T001 — the 14 `pr_*.txt` files and `pr_README.md` added (11 recorded with `gh` 2.54.0, 3
  written from GitHub's documented answer). `mise run test-core` -> 1511 passed, 0 failed, 7 ignored
- refactor: none needed
- notes: the first green run of the single target failed to build (`crate unicode_ident required to
  be available in rlib format`): another worktree was writing the shared target directory. The
  suite run right after it built and passed; nothing in this tree changed between the two.
- commit: the commit that adds this entry

## Cycle 2: U46, U47 — a pull request's status is never stored and never printed whole

- test: `crates/micold-core/tests/pull_request_is_never_stored.rs` (new), 2 tests
- stub: new `src/pull_request.rs` (`pub mod pull_request;`) declaring `PullRequestStatus`,
  `PrState`, `CheckStatus` and `ReviewState`, each with `#[derive(Debug, Clone, PartialEq, Eq)]`
- red: `scripts/build-lock.sh cargo test -p micold-core --test pull_request_is_never_stored`
  -> `test result: FAILED. 0 passed; 2 failed`. Decisive lines:
  `the_status_derives_no_serialisation_and_no_debug`: `PullRequestStatus must not derive Debug: a
  derived one would write the title and the address to a stored file or a log (FR-032); found: …
  #[derive(Debug, Clone, PartialEq, Eq)]`;
  `debug_output_holds_the_number_and_the_enums_and_neither_title_nor_address`: `the title is never
  printed (FR-032): PullRequestStatus { number: 4711, title: "Rework the billing export", url:
  "https://github.com/acme/widgets/pull/4711", state: Open { checks: Failing }, review:
  ChangesRequested, head: "0123…4567" }`
- green: T009 — `PullRequestStatus` derives `Clone, PartialEq, Eq` only and has a hand-written
  `Debug` (`number`, `state`, `review`, `finish_non_exhaustive`); `ReadingFailure` declared beside
  the enums. -> `2 passed; 0 failed`; `mise run test-core` 1513 passed, 0 failed, 7 ignored
- refactor: none needed
- commit: the commit that adds this entry

## Notes and deviations (M1, cycles 3 to 8)

- **One stub, six reds, one green.** Cycles 3 to 8 share one stub and one implementation commit. The
  stub declared every symbol of contracts/pull-request-source.md with an answer chosen to be wrong
  for every test: `status_query` the empty string, `status_args` no argument, `split_response` a
  status 0 with no header and an empty body, `parse_status`, `reading_failure`, `read_outcome`,
  `read_in_chunks` and the fake's `read` `Err(RateLimited { until: 0 })`, `select_pull_request` a
  closed pull request numbered 0, `reduce_checks` `Failing` for no count and `None` for any count,
  `rate_limit_pause` 0. Each file's tests were then written and run against it; **every test of the
  six files failed at red, none passed**, so no deliberate-mutant check was needed. The red runs
  were `scripts/build-lock.sh cargo test -p micold-core --test <file> --no-fail-fast`.
- **Three reds failed one step early.** `parse_status_missing_alias_is_passing`,
  `parse_status_null_repository_is_passing` and `parse_status_graphql_errors_are_passing` build
  their input from a fixture's body, and the stub's `split_response` returns an empty one, so at
  red they stopped at `fixture body is JSON: EOF while parsing a value`, not at their own
  `Err(Passing)` assertion. `a_failed_run_is_its_failure` (four `read_outcome` cases) stopped at
  its first case. All reach their own assertions at green.
- **A test's own mistake, fixed at green.** `u3_query_for_fifty_branches_is_pinned` passed its pin
  comparison and then failed on a check written too widely (`": String!"` must be absent, which
  `$owner: String!` breaks). The check now names `$b50: String!`. The implementation was not
  changed for it.
- **The pin of `status_query(50)`** is `crates/micold-core/tests/fixtures/gh/status_query_50.graphql`,
  written by a generator independent of the Rust code from the recorded one-branch document, and
  sent to GitHub once for 50 branch names that do not exist: exit 0, no `errors`,
  `rateLimit { remaining: 4603 }`.
- **Beyond the test list.** `read_outcome` (one `gh` run to an answer or a failure kind) is the
  pure half of `GhCli::read`, so `pull_request_source.rs` holds two tests of it; the review
  mapping and "a count of 0 does not count" have a test each in the select and checks files.

## Cycle 3: U2–U6 — the query and the arguments

- test: `crates/micold-core/tests/pull_request_query.rs` (new), 5 tests
- red: `0 passed; 5 failed`. Decisive lines: `u2_…`: `assert_eq!(q, QUERY_1)` left `""`; `u3_…`:
  left `""`, right the pinned document; `u4_…`: `U4: status_args must hold a query= argument`;
  `u5_…`: `hits.len()` left 0, right 1 for `b0=feat/"quoted"`; `u6_…`: left `[]`, right the 15
  arguments of the contract
- green: T010 — `status_query`, `status_args`. `5 passed; 0 failed`
- refactor: none needed

## Cycle 4: U23–U30 — the combined check status

- test: `crates/micold-core/tests/pull_request_checks.rs` (new), 8 tests (U23 to U25 are one
  table of 19 rows)
- red: `0 passed; 8 failed` (the stub answers `Failing` without counts and `None` with any)
- green: T011 — `reduce_checks`, `CheckCounts`. `8 passed; 0 failed`
- refactor: none needed

## Cycle 5: U16–U22 — which pull request a branch is shown with

- test: `crates/micold-core/tests/pull_request_select.rs` (new), 11 tests
- red: `0 passed; 11 failed`. Decisive lines: U16 `status.number` left 0, right 1; U20
  `Ok(Some(PullRequestStatus { number: 0, state: Closed, .. }))` where `Ok(None)` is expected; U22
  the same where `Err(Unreadable)` is expected
- green: T011 — `select_pull_request`, `PrNode`, `Unreadable`. `11 passed; 0 failed`
- refactor: none needed

## Cycle 6: U7–U15 — reading the recorded answers

- test: `crates/micold-core/tests/pull_request_parse.rs`, 19 tests added to U1's
- red: `1 passed; 19 failed` (the one is U1, green since cycle 1). Decisive lines: U7 `status comes
  from the status line` left 0, right 200; U8 `no status line means no answer (FR-019)`; U9
  `pr_three_branches.txt is a readable answer, got RateLimited { until: 0 }`; U13 left
  `Err(RateLimited { until: 0 })`, right `Err(Passing)`
- green: T011 — `split_response`, `Response`, `parse_status`. `20 passed; 0 failed`. T011 is
  complete with cycles 4 to 6
- refactor: none needed

## Cycle 7: U31–U41 — the kind of a failure, and how long a rate limit pauses

- test: `crates/micold-core/tests/pull_request_failure.rs` (new), 26 tests, on T001's answers and
  034's `*.stderr` fixtures
- red: `0 passed; 26 failed`: every `reading_failure` case left `RateLimited { until: 0 }`, every
  `rate_limit_pause` case left 0
- green: T012 — `rate_limit_pause`, `reading_failure` on top of `github::classify`.
  `26 passed; 0 failed`
- refactor: none needed

## Cycle 8: U42–U45 — the source, its fake and the chunks

- test: `crates/micold-core/tests/pull_request_source.rs` (new), 7 tests
- red: `0 passed; 7 failed`. Decisive lines: U42 the first `read` answers
  `Err(RateLimited { until: 0 })` where the scripted map is expected; U43 the same where `Ok({})`
  is; U44 `50 branches are asked for in chunks of [50]` left `[]`; U45 left
  `Err(RateLimited { until: 0 })`, right `Err(Unavailable)`
- green: T013 — `PullRequestSource`, `FakePullRequestSource`, `read_in_chunks`, `read_outcome`, and
  `impl PullRequestSource for GhCli` over the runner `GhCli::run` uses (`GhCli::outcome`, split out
  of it). `7 passed; 0 failed`
- refactor: none needed
- suite: `mise run test-core` -> 1589 passed, 0 failed, 7 ignored (142 test-result lines), exit 0:
  the 1513 of cycle 2 and the 76 tests of cycles 3 to 8
- commit: the commit that adds this entry

## Cycle 9: U33 — a rate limit beside data that cannot be read (review A, F1)

- test: `crates/micold-core/tests/pull_request_failure.rs::a_rate_limit_beside_unreadable_data_is_still_a_rate_limit` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test pull_request_failure` on the source
  of cycle 8 -> `26 passed; 1 failed`: `left: Passing`, `right: RateLimited { until: 1790966100 }`
  (`reading_failure` read `errors` through the strict `Answer`, so a `null` node dropped them)
- green: `reading_failure` reads `errors` through `AnswerErrors`, which holds nothing else.
  `27 passed; 0 failed`
- refactor: none needed
- also in this commit, untested (it needs a real `gh` and a clock): `GhCli::read` adds the time
  since the reading began to `now` for each chunk, so a later chunk's `Retry-After` counts from its
  own answer (review A, F2)
- suite: `mise run gate` (the workspace suite) before the pull request
- commit: the commit that adds this entry

## M2 — protocol 21: the switch and the merged-branch question (U48–U69)

### M2: notes and deviations

- **Prepared ahead of M1's merge.** These cycles ran on a branch cut from `47f73eb1` (the design
  merge), beside M1's branch, and use nothing of M1's code. Suite counts are therefore counted
  from the baseline above (1510), not from M1's last cycle.
- **One behaviour per cycle, run as one batch.** The build lock was held by other worktrees for
  most of the session, so the tree of every step — test written, then implementation — was recorded
  as a git tree object first, and one run under one lock then walked them in order: restore the
  red tree, run the one test (`-- --exact`) and require it to fail; restore the green tree, run it
  again and require `1 passed`; run the suite. The run stops at the first step that does not go as
  recorded. Every red below is that run's own output for that step. Refactors were therefore
  limited to what the green tree already held.
- **Command.** red and green: `scripts/build-lock.sh cargo test --test <target> <test> -- --exact`.
  suite: for core cycles `scripts/build-lock.sh cargo test -p micold-core --all-targets`
  (`mise run test-core`); for service cycles `scripts/build-lock.sh cargo test --test
  merged_branch_check --test daemon_lifecycle` (plus `--test pr_status_setting` from U67). The
  whole workspace runs in `mise run gate` at the end of the milestone.
- **Compile-error reds.** U48, U56 and U57 fail because the field or the variant under test does
  not exist; there is no stub that would make them fail on an assertion without being the
  implementation.
- **Tests that needed no new code** (U61, U63, U65, U66, U69): an earlier cycle's implementation
  already covered them, so each was held to a deliberate mutant. The batch ran the test against the
  mutant first (the `red` of those entries) and then against the unchanged implementation (the
  green); the mutant was never committed.
- **A discarded run.** The first attempt at U48's red failed with `failed to build archive …
  libsyn … No such file or directory`: another session swept the shared target directory during
  the build. It is not counted as a red. A second attempt of the batch stopped at U48's suite
  because restoring a tree left a later cycle's new test file in place; the restore was fixed
  (`git read-tree -u --reset`) and the batch run again from U48. The reds below are from that
  third run.
- **`BranchContainment` before the bump.** `containment` returns it, so the enum entered
  `protocol/messages.rs` at U50, while `PROTOCOL_VERSION` moved at U58. The schema hash differs
  between those commits and the version does not; `tests/schema_hash.rs` pins the version only, and
  the milestone ships as one change with one bump.
- **A pin updated on purpose.** `tests/settings_issue_mapping.rs` lists the keys the settings file
  may hold; `pr_status_enabled` was added to that list in U49's green.
- **A second pin, found by the gate.** `crates/micold-client/tests/settings_sections.rs` requires
  every stored setting to have a control or a `DEFERRED` entry naming the task that adds one. The
  first gate run failed there (`these persisted settings are rendered by no section and recorded
  as deferred by nothing: ["pr_status_enabled"]`); the entry `("pr_status_enabled", "040 T038")`
  was added in the commit after U69's. M4's T038 removes it when the checkbox arrives.
- **Formatting.** The per-cycle commits hold the trees the batch ran; `cargo fmt` changed four
  files afterwards, in that same later commit.
- **The client keeps the stored switch.** The Settings form does not have the switch until M4, so
  `shell/persist.rs` keeps the stored `pr_status_enabled` across a save and sends `None` in
  `SettingsSet` (U48, U57; T022). `daemon_sync.rs` reads `DaemonSettings` by field and needed no
  change.

### M2 cycle: U48 — the switch is off by default and in a file written before it

- test: `crates/micold-core/tests/settings_roundtrip.rs::pull_request_status_is_off_by_default_and_in_a_file_written_before_the_switch`
- red: `error[E0609]: no field `pr_status_enabled` on type `Settings`` (twice: the default, and the
  loaded settings)
- green: `pr_status_enabled: bool` on `Settings` (`#[serde(default)]`, default `false`) and on the
  stored form, read back as `false`; the client's two `Settings` literals carry it. Suite: 1511
  passed, 0 failed, 7 ignored
- refactor: none needed
- commit: `b578c1ae`

### M2 cycle: U49 — on survives a save and a load, at the same settings version

- test: `crates/micold-core/tests/settings_roundtrip.rs::turning_pull_request_status_on_survives_a_save_and_load_at_the_same_settings_version`
- red: `panicked at crates/micold-core/tests/settings_roundtrip.rs:693:5: the user turned the
  switch on; loading the file must not turn it off` (`test result: FAILED. 0 passed; 1 failed`)
- green: the stored form's value is carried into `Settings` on load; `SETTINGS_VERSION` stays 4.
  Suite: 1512 passed, 0 failed, 7 ignored
- refactor: none needed
- commit: `3e17166a`

### M2 cycle: U50 — a tip equal to the head, or an ancestor of it, is contained

- test: `crates/micold-core/tests/git_containment.rs::a_tip_equal_to_the_head_or_an_ancestor_of_it_is_contained` (new file)
- stub: `BranchContainment` and a `containment` that returns `Unknown`
- red: `assertion `left == right` failed: a tip equal to the head is contained without asking git
  about ancestry  left: Unknown  right: Contained`
- green: `containment` answers `Contained` for an equal tip and for `Some(true)`. Suite: 1513
  passed, 0 failed, 7 ignored
- refactor: none needed
- commit: `54a61826`

### M2 cycle: U51 — a tip that is not an ancestor of the head is beyond

- test: `crates/micold-core/tests/git_containment.rs::a_tip_that_is_not_an_ancestor_of_the_head_is_beyond`
- red: `assertion `left == right` failed: a branch with commits after its merged pull request is
  beyond it  left: Unknown  right: Beyond`
- green: `Some(false)` is `Beyond`. Suite: 1514 passed, 0 failed, 7 ignored
- refactor: none needed
- commit: `51e801c1`

### M2 cycle: U52 — no tip, or unknown ancestry, is unknown

- test: `crates/micold-core/tests/git_containment.rs::no_tip_or_unknown_ancestry_is_unknown`
- red: `assertion `left == right` failed: without a tip there is nothing an ancestry answer could
  be about  left: Contained  right: Unknown`
- green: no tip is `Unknown` before the ancestry is looked at. Suite: 1515 passed, 0 failed, 7
  ignored
- refactor: none needed
- commit: `761dd602`

### M2 cycle: U53 — git reads a branch's tip, and none for a missing branch

- test: `crates/micold-core/tests/git_containment.rs::the_real_git_reads_a_branch_s_tip_and_none_for_a_missing_branch`
- stub: `Git::branch_tip` and `Git::is_ancestor` returning `None`
- red: `assertion `left == right` failed: the tip is the full id of the commit the branch points at
  left: None  right: Some("145a94eaac9be8b2ecdafb68ba33ac78f1ef0068")`
- green: `GitCli::branch_tip` runs `git rev-parse --verify --quiet refs/heads/<branch>^{commit}`.
  Suite: 1516 passed, 0 failed, 7 ignored
- refactor: none needed
- commit: `0905c4ca`

### M2 cycle: U54 — git tells an ancestor from a descendant and from a commit it does not hold

- test: `crates/micold-core/tests/git_containment.rs::the_real_git_tells_an_ancestor_from_a_descendant_and_from_a_commit_it_does_not_hold`
- red: `assertion `left == right` failed: a branch at the head  left: None  right: Some(true)`
- green: `GitCli::is_ancestor` runs `git merge-base --is-ancestor`: exit 0 is `Some(true)`, exit 1
  is `Some(false)`, anything else is `None`. Suite: 1517 passed, 0 failed, 7 ignored
- refactor: none needed
- commit: `977dda8e`

### M2 cycle: U55 — the fake git answers the tip and the ancestry as scripted

- test: `crates/micold-core/tests/git_containment.rs::the_fake_git_answers_the_tip_and_the_ancestry_as_scripted`
- stub: `FakeGit::with_branch_tip` and `FakeGit::with_ancestry` that record nothing
- red: `assertion `left == right` failed  left: None  right:
  Some("1111111111111111111111111111111111111111")`
- green: `FakeGit` keeps both scripts and answers from them, `None` when nothing was scripted.
  Suite: 1518 passed, 0 failed, 7 ignored
- refactor: none needed
- commit: `c1ce0b26`

### M2 cycle: U56 — the merged-branch question and its answers round-trip in order

- test: `crates/micold-core/tests/protocol_roundtrip.rs::a_merged_branch_check_and_its_answers_round_trip_in_order`
- red: `error[E0432]: unresolved import `micold_core::protocol::messages::MergedBranchQuery``;
  `error[E0599]: no variant named `MergedBranchCheck` found for enum `ClientMsg``; the same for
  `OperationResult`
- green: `MergedBranchQuery`, `ClientMsg::MergedBranchCheck` and
  `OperationResult::MergedBranchCheck`. Suite: 1519 passed, 0 failed, 7 ignored
- refactor: none needed
- commit: `44c7a16e`

### M2 cycle: U57 — the switch round-trips in `DaemonSettings` and `SettingsSet`

- test: `crates/micold-core/tests/protocol_roundtrip.rs::the_pull_request_switch_round_trips_in_daemon_settings_and_settings_set`
- red: `error[E0559]: variant `ClientMsg::SettingsSet` has no field named `pr_status_enabled``;
  `error[E0560]: struct `DaemonSettings` has no field named `pr_status_enabled``
- green: the field on both; every literal in the service, the client and their tests names it (the
  service ignores it until U67, the client sends `None`). Suite: 1520 passed, 0 failed, 7 ignored
- refactor: none needed
- commit: `a2efd5cb`

### M2 cycle: U58 — one bump, to 21, that covers all of it

- test: `crates/micold-core/tests/schema_hash.rs::the_merged_branch_question_and_the_pull_request_switch_cost_one_bump_to_21`
  (and the existing pin moved to 21)
- red: `assertion `left == right` failed: feature 040's wire change is one bump, 20 → 21; …
  left: 20  right: 21`
- green: `PROTOCOL_VERSION` is 21. Suite: 1521 passed, 0 failed, 7 ignored
- refactor: none needed
- commit: `58e23cd8`

### M2 cycle: U59 — a branch at or behind the head is contained

- test: `crates/micold-daemon/tests/merged_branch_check.rs::a_branch_at_or_behind_the_head_is_contained` (new file)
- red: `panicked at crates/micold-daemon/tests/merged_branch_check.rs:144:10: the reply arrives:
  Elapsed(())` — the service does not answer the message (`finished in 30.05s`)
- green: fake it — the arm answers `Contained` for every query. Suite: 8 passed, 0 failed
- refactor: none needed
- commit: `3bff4329`

### M2 cycle: U60 — a commit after the head is beyond

- test: `crates/micold-daemon/tests/merged_branch_check.rs::a_branch_with_a_commit_after_the_head_is_beyond`
- red: `assertion `left == right` failed  left: [Contained]  right: [Beyond]`
- green: the real arm, a copy of `RemoteList`'s: the project's repository or the refusal, then
  `branch_tip`, `is_ancestor` and `containment` per query on a blocking thread. Suite: 9 passed, 0
  failed
- refactor: none needed (the per-query answer was written as its own function,
  `merged_branch_answer`)
- commit: `922cf13d`

### M2 cycle: U61 — a missing branch and a head never fetched are unknown

- test: `crates/micold-daemon/tests/merged_branch_check.rs::a_missing_branch_and_a_head_that_is_not_a_local_object_are_unknown`
- passed on its first run against U60's arm. Mutant: `Unknown` answered as `Contained`
- red (mutant): `assertion `left == right` failed: a missing branch, then a head the repository
  does not hold  left: [Contained, Contained]  right: [Unknown, Unknown]`
- green: mutant removed, no source change. Suite: 10 passed, 0 failed
- refactor: none needed
- commit: `07cac01e`

### M2 cycle: U62 — a head that is not a full commit id never reaches git

- test: `crates/micold-daemon/tests/merged_branch_check.rs::a_head_that_is_not_a_full_commit_id_is_unknown_without_running_git`
- red: `assertion `left == right` failed: the full id is answered from the repository; a ref name,
  a branch name, an abbreviated id and an empty head are not passed to git  left: [Contained,
  Contained, Contained, Contained, Unknown]  right: [Contained, Unknown, Unknown, Unknown, Unknown]`
  — git resolved `HEAD`, a branch name and an abbreviated id
- green: a head is used only when it is 40 or 64 hexadecimal characters. Suite: 11 passed, 0
  failed
- refactor: none needed
- commit: `083576fa`

### M2 cycle: U63 — one answer per query, in query order

- test: `crates/micold-daemon/tests/merged_branch_check.rs::the_answers_are_one_per_query_in_query_order`
- passed on its first run. Mutant: the queries answered in reverse
- red (mutant): `assertion `left == right` failed  left: [Contained, Unknown, Contained, Beyond]
  right: [Beyond, Contained, Unknown, Contained]`
- green: mutant removed, no source change. Suite: 12 passed, 0 failed
- refactor: none needed
- commit: `2ddd90bd`

### M2 cycle: U64 — 50 queries are answered and 51 are refused

- test: `crates/micold-daemon/tests/merged_branch_check.rs::fifty_queries_are_answered_and_fifty_one_are_refused`
- red: `51 queries must be refused, got OperationOk { req: 7, result: MergedBranchCheck { answers:
  [Contained, Contained, …`
- green: `MERGED_BRANCH_CHECK_LIMIT = 50`; a longer list is `ErrorKind::InvalidInput`. Suite: 13
  passed, 0 failed
- refactor: none needed
- commit: `69d0b7cb`

### M2 cycle: U65 — a project that is not a repository is refused

- test: `crates/micold-daemon/tests/merged_branch_check.rs::a_project_that_is_not_a_repository_is_refused`
- passed on its first run (the arm took `reject_non_repo` from `RemoteList`'s at U60). Mutant: the
  arm accepts a project that is not a repository
- red (mutant): `a folder that is not a repository must be refused, got OperationOk { req: 7,
  result: MergedBranchCheck { answers: [Unknown] } }`
- green: mutant removed, no source change. Suite: 14 passed, 0 failed
- refactor: none needed
- commit: `97480e66`

### M2 cycle: U66 — the check leaves the repository as it found it

- test: `crates/micold-daemon/tests/merged_branch_check.rs::the_check_leaves_the_repository_as_it_found_it`
  (every file under the project, `.git` included, compared by name and content)
- passed on its first run. Mutant: the answer writes `.git/FETCH_HEAD`
- red (mutant): `assertion `left == right` failed: the check added or removed a file` (the right
  side lists `.git/FETCH_HEAD`)
- green: mutant removed, no source change. Suite: 15 passed, 0 failed
- refactor: none needed
- commit: `d256f03e`

### M2 cycle: U67 — the switch is persisted and reported in the next `Welcome`

- test: `crates/micold-daemon/tests/pr_status_setting.rs::turning_pull_request_status_on_is_persisted_and_reported_in_the_next_welcome` (new file)
- red: `panicked at crates/micold-daemon/tests/pr_status_setting.rs:141:5: the next client to
  connect is told the switch is on`
- green: `Catalog::set_pr_status_enabled` (written to the settings file with the other service
  settings), `DaemonState::set_pr_status_enabled`, and the `SettingsSet` arm calls it for `Some`.
  Suite: 16 passed, 0 failed
- refactor: none needed
- commit: `437fb494`

### M2 cycle: U68 — the change is broadcast to two connected clients

- test: `crates/micold-daemon/tests/pr_status_setting.rs::turning_pull_request_status_on_is_broadcast_to_two_connected_clients`
- red: `the client was never told the settings changed: Elapsed(())` (`finished in 5.00s`)
- green: `DaemonState::set_pr_status_enabled` broadcasts `SettingsChanged`, as
  `set_tool_server_enabled` does. Suite: 17 passed, 0 failed
- refactor: none needed
- commit: `64cc6178`

### M2 cycle: U69 — a change that does not name the switch leaves it as it is

- test: `crates/micold-daemon/tests/pr_status_setting.rs::a_settings_change_that_does_not_name_the_switch_leaves_it_as_it_is`
- passed on its first run. Mutant: `None` turns the switch off
- red (mutant): `panicked at crates/micold-daemon/tests/pr_status_setting.rs:223:5: nor in the
  settings file: a restarted service still has it on`
- green: mutant removed, no source change. Suite: 18 passed, 0 failed
- refactor: none needed
- commit: `3ca8ffd9`

### M2 cycle: U61 (review A) — a branch name is only a name under `refs/heads/`

- red: `a_branch_name_with_a_revision_suffix_is_unknown` in `crates/micold-daemon/tests/merged_branch_check.rs`
  failed on the cherry-picked M2 code: `left: [Contained, Contained, Contained]`, `right: [Unknown, Unknown, Unknown]`
  (`ahead~1`, `ahead^`, `ahead^{commit}~1` resolved to the merged commit and hid `ahead`'s newer one)
- green: `is_plain_branch_name` in `crates/micold-daemon/src/server.rs` answers `Unknown` for a name git would
  read as revision syntax; `merged_branch_check` 9 passed, `pr_status_setting` 3 passed
- refactor (same review): the `MergedBranchCheck` arm is spawned, not awaited, so its git calls never hold up the
  connection loop's `Ping` (BUG-009); its join failure uses `task_failed`; `GitCli::is_ancestor` sets
  `GIT_NO_LAZY_FETCH=1` and `GIT_TERMINAL_PROMPT=0` so a partial clone never fetches (FR-016)
- suite: `mise run gate` before the pull request
- commit: `cd8c5eb3`

### M2 refactor: review A round 2 — the branch guard lives in `GitCli::branch_tip`

- the revision-syntax guard moved from the daemon into `GitCli::branch_tip`, reusing
  `naming::is_valid_branch` (now `pub(crate)`) instead of a second copy of the ref-format rules
- `branch_tip` and `is_ancestor` both run through `local_only` (`GIT_NO_LAZY_FETCH=1`, git 2.44+;
  `GIT_TERMINAL_PROMPT=0`)
- `a_branch_name_with_a_revision_suffix_is_unknown` also asks for `ahead@{0}~1`
- green: `merged_branch_check`, `pr_status_setting`, `git_containment` in the gate
- commit: `800cfd82`

### M2 cycle: U48 (review B) — a Settings save keeps the stored pull request switch

- red: `a_settings_save_keeps_the_stored_pr_status_switch` in `crates/micold-client/src/main_tests.rs`, run with
  `persist.rs`'s `stored.pr_status_enabled = pr_status_enabled;` removed: `left: Some(false)`, `right: Some(true)`
- green: the line restored; the test passes
- also (review B): the M2 `commit:` hashes above are the ones on `feat/worktree-pr-ci-status` after the
  cherry-pick, not the preparation branch's
- suite: `mise run gate` before the pull request
- commit: the commit that adds this entry

### M3 cycle: U70–U84 — the schedule reducer (T023, T026)

- red: `crates/micold-client/tests/features_pr_status.rs` against the stub `update` (commit `f8471596`'s tree, which
  returns `Effect::None`): 3 passed, 13 failed, each on an assertion, e.g.
  `held_then_listing_reads_once_and_a_second_listing_reads_nothing` at `features_pr_status.rs:105` and
  `reading()`'s `held and listed with the switch on starts a reading` at `:68`
- green: `features::pr_status::update` per reading-and-wire §2 and data-model "State transitions"; 16 passed
- test fix on the way to green: the helper `reading_again` saved the statuses after switching off, which had
  already cleared them; it now saves them (and `read_at`) before. `a_passing_failure_changes_nothing` and
  `a_rate_limit_keeps_the_statuses_and_pauses` failed on that, `left: {}`, not on the reducer
- refactor: `start` and `paused` hold the §1 conditions once, for S1, S2 and the further reading
- commit: `feat(040): the pull request reading's schedule reducer (T026)`

### M3 cycle: U85–U97, A10–A14, A31, A40–A42 — the reading through the shell (T025, T027–T029)

- tests: `pr_status_*` in `crates/micold-client/src/main_tests.rs` (16), with `FakePullRequestSource`
- written after `shell/pr_status.rs` and the wiring (the handover's order), so they passed on their first run:
  16 passed. Red by mutant, each run on its own:
  - `branches()` without the deduplication:
    `pr_status_reads_the_listed_branches_after_attached_and_the_listing` fails
  - `Displaced` not sending `Released`: `pr_status_displaced_clears_and_reads_nothing` and
    `pr_status_a_take_over_reads_once` fail
- green: mutants removed; `cargo test -p micold-client pr_status` 16 passed
- guards that failed first and were answered in the code: `feature_registration_cost` (the shell half names
  `features::pr_status::Msg`), `no_concrete_implementations` (`GhCli` is chosen once in `Capabilities::real()`)
- commit: the commit that adds this entry

### M3 cycle: U85–U88 — the source gate (T024)

- test: `crates/micold-client/tests/pr_status_is_read_only_on_named_events.rs`, after
  `issues_are_requested_only_on_named_events.rs`; a gate over the source text, green on the tree it was written
  against (it pins what T028 and T029 built)
- commit: the commit that adds this entry

### M4 cycle: U98–U110 — the row indicator, the icons, the switch (T030–T039)

- test: red first in the commit before this one (`row_pull_request`, the seven `Icon` variants, `GithubDraft.pr_status_enabled`,
  `Msg::PrStatusToggled`, `SettingsSet` carrying the switch only when changed, the showcase entry); none compiled until the code existed
- green: `icons.rs` (D13: `edit_note`, `cancel`), `ui/material/pull_request_indicator.rs` (width 16/34 unit test),
  `features::sidebar::row_pull_request`, the row's trailing element, the Settings switch and `persist.rs`
- covered states: `main-shell-sidebar-pull-request-indicator` (+ `-narrowest`) and the checked switch in
  `settings-view-github-issues`; the showcase is not a covered-state kind, its entry is held by `showcase_completeness`
- commit: the commit that adds this entry

### M5 cycle: U111–U118 — tooltip lines, Open pull request (T041–T047)

- test: written before the code. The tooltip tests (`features_sidebar.rs`) and `worktree_menu_pull_request.rs` ran red on their
  assertions against `worktree_tooltip` / `worktree_menu_items` signature stubs taking the new arguments and ignoring them (5 of
  the 8 tooltip tests failed; the menu tests were not reached because cargo stops at the first red binary, and were written against the
  same stub). `pr_status_open_*` (`main_tests.rs`) were written before the handler against the `PullRequestOpenRequested` variant
  alone, then run once with the handler in place (not seen red on their own: deviation, the variant stub existed only in my head).
  The `OpenInBrowser` icon was added in the same step as its tests (not seen red: deviation).
- green: `worktree_tooltip` lines 1 to 4, `worktree_menu_items` entry, `shell/pr_status.rs::open_requested` over
  `features::sidebar::pull_request_address_to_open` (the `https://github.com/` guard, pure so no `.url` read in the two scanned files),
  `links::perform` made `pub(crate)`, the user guide
- commit: the commit that adds this entry
