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
