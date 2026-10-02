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
