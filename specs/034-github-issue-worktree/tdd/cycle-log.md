# Cycle Log: Create a Worktree from a GitHub Issue

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> not yet measured
- commit: `d6c2f33e` (test list planned against it)
- recorded: cycle 0. The design PR changes no code, so the full suite is not run for it. M1 starts
  from a fresh `origin/main` after this PR merges; its first entry below runs the suite on that
  base and records the counts before any red.

## Baseline (measured, M1)

- suite: `scripts/build-lock.sh cargo test --workspace` -> 3662 passed, 0 failed, 8 ignored, 341
  binaries (exit 0)
- commit: `13967080` (`origin/main` with design PR #459 merged)
- recorded: before any M1 change

## Notes and deviations (M1)

- **Batched reds, per test file.** As in feature 033's M1: every test of one file is written first,
  run against the code as it stands (or a stub that only makes the symbols resolve), and each is
  observed failing on its own assertion. A test that passed at that point is held to the
  deliberate-mutant check after green. The inner loop runs the one test target; the per-cycle suite
  is `mise run test-core` (the core crate, where every M1 behaviour lives); the full workspace suite
  runs in `mise run gate` before the PR.
- **Structural move first.** T010's move of `run_bounded` / `RunOutcome` / `kill_process_group` to
  `process.rs` is behaviour-preserving, so it went in as its own refactor commit on green
  (`refactor(034): move the bounded runner…`, `mise run test-core` 1264 passed, 0 failed) before
  cycle 1's red. That is also U5's baseline: the env-include suites passed unchanged across it.

## Cycle 1: U1–U5 — the bounded runner drains its pipes

- test: `crates/micold-core/tests/process_run_bounded.rs` (new), 4 tests plus `child_helper` (the
  child is the test binary re-run with `MICOLD_RUN_BOUNDED_CHILD`, so no shell on any OS)
- red: `scripts/build-lock.sh cargo test -p micold-core --test process_run_bounded`
  -> `test result: FAILED. 4 passed; 1 failed`. Decisive line,
  `large_output_is_drained_while_waiting`: `output larger than a pipe buffer must not stall the
  child into a timeout, got TimedOut { stderr: "" } after 5.001466872s`.
  Passed before the change (behaviour the moved runner already had): U1
  `a_child_past_the_bound_is_killed_and_reported`, U2 `a_child_inside_the_bound_exits_normally`,
  U4 `a_failing_child_reports_status_and_output`.
- green: `process::run_bounded` reads stdout and stderr on two `drain` threads started right after
  spawn and joins them after the group kill; stderr is decoded lossily. -> `5 passed; 0 failed`;
  `mise run test-core` 1269 passed, 0 failed (U5: `env_include*.rs` unchanged and green)
- mutant check (the three that passed first): group kill removed -> U1 FAILED; poll bound replaced
  by `Duration::ZERO` -> U2 and U4 FAILED; exit code forced to 0 -> U4 FAILED. Code restored each
  time, 5 passed again.
- refactor: none needed
- commit: the commit that adds this entry

## Cycle 2: U6–U10, U97 — a repository's own remotes

- test: `crates/micold-core/tests/git_remotes.rs` (new), 6 tests
- stub: `GitRemote`, `parse_remote_list` returning `[]`, `Git::remote_list` returning `Ok("")` on
  both `GitCli` and `FakeGit`, `FakeGit::with_remote` recording nothing
- red: `scripts/build-lock.sh cargo test -p micold-core --test git_remotes`
  -> `test result: FAILED. 0 passed; 6 failed`. Decisive lines, e.g.
  `a_dotted_remote_name_is_kept_whole`: `the name is everything between remote. and the last .url /
  left: [] / right: [GitRemote { name: "a.b", … }]`; `global_insteadof_is_not_applied`: `the
  repository's own URL is listed as written … / left: [] / right: [GitRemote { name: "origin",
  url: "gh:o/r" }]`; `git_cli_lists_remotes_and_none_is_not_an_error`: `left: []`.
- green: `parse_remote_list` (split on the first space, strip `remote.` / `.url`, first URL per
  name); `GitCli::remote_list` runs `config --local --get-regexp ^remote\..+\.url$` and maps exit 1
  to `Ok("")`; `FakeGit` keeps remotes per repo in insertion order. -> `6 passed; 0 failed`;
  `mise run test-core` 1275 passed, 0 failed
- refactor: none needed
- notes: U97 appended to the list — T011 names `FakeGit::with_remote`, which no listed behaviour
  covered
- commit: the commit that adds this entry

## Cycle 3: U11–U16 — the GitHub repository behind a remote

- test: `crates/micold-core/tests/github_remote.rs` (new), 6 tests
- stub: new `github.rs` (`pub mod github;`) with `GithubRepo::from_remote_url` returning `None` and
  `choose_remote` returning `NoGithubRemote`
- red: `scripts/build-lock.sh cargo test -p micold-core --test github_remote`
  -> `test result: FAILED. 2 passed; 4 failed`. Decisive lines: `accepted_url_forms`:
  `` `https://github.com/o/r` is a github.com remote naming o/r / left: None / right: Some("o/r") ``;
  `userinfo_is_discarded`: `left: None / right: Some("o/r")`; `origin_wins_when_on_github`: `origin
  on GitHub is chosen over an earlier GitHub remote, got NoGithubRemote`;
  `first_github_remote_otherwise`: `origin not on GitHub: the GitHub remote is chosen`.
  Passed on the stub: U13 `non_github_urls_are_rejected`, U16 `no_github_remote` (both assert a
  rejection).
- green: `from_remote_url` splits scheme URLs (`http(s)`, `git`, `ssh`) into authority and path,
  drops userinfo and a numeric port, and reads the scp form `[user@]host:path`; the host must be
  `github.com` (or `ssh.github.com` over SSH), the path exactly `owner/name` after `.git` and `/`
  are stripped. `choose_remote` takes `origin` when it parses, else the first remote that does.
  -> `6 passed; 0 failed`; `mise run test-core` 1281 passed, 0 failed
- mutant check: host check removed -> U13, U15 and U16 FAILED; `choose_remote` falling back to a
  made-up GitHub remote instead of `NoGithubRemote` -> U16 FAILED. Restored, 6 passed again.
- refactor: none needed
- commit: the commit that adds this entry

## Cycle 4: U17–U23 — finding `gh` as a desktop-launched app

- test: `crates/micold-core/tests/github_locate.rs` (new), 7 tests, all against a fake `exists`
- stub: `HostOs` (`exe_name` `""`, `path_separator` `'\0'`), `LocateInputs`, `env_include_path`,
  `candidate_dirs` and `locate_gh` returning nothing
- red: `scripts/build-lock.sh cargo test -p micold-core --test github_locate`
  -> `test result: FAILED. 1 passed; 6 failed`. Decisive lines:
  `separator_and_exe_come_from_host_os`: `left: ('\0', "") / right: (':', "gh")`;
  `macos_dock_launch_finds_homebrew_gh`: `a Dock launch cannot see Homebrew on its PATH, so the
  well-known table finds it / left: None / right: Some("/opt/homebrew/bin/gh")`;
  `windows_finds_winget_gh`: `left: None`; `linux_well_known_dirs`: `/usr/local/bin is searched on
  Linux: []`; `path_key_is_matched_ignoring_case`: `Windows spells the key Path / left: None`.
  `candidate_order` first failed with `range end index 3 out of range for slice of length 0` — a
  slicing panic, not an assertion, so the test was changed to compare the first three candidates
  as a `Vec` (a test fix before any implementation) and re-run:
  `env-include PATH first, then process PATH; … / left: [] / right: ["/from/profile", "/shared",
  "/from/process"]`. Passed on the stub: U23 `none_when_absent`.
- green: `HostOs` separator/exe name/`current()` (the one `cfg!`), the per-OS well-known table of
  research R3 (Windows entries built as `\`-joined text from their variables, skipped when unset),
  `env_include_path` matching `PATH` ignoring ASCII case, `candidate_dirs` (env-include, process,
  well-known; deduplicated, empties dropped), `locate_gh` probing `dir/exe_name`.
  -> `7 passed; 0 failed`; `mise run test-core` 1288 passed, 0 failed
- mutant check: `locate_gh` accepting the first candidate whatever `exists` says -> U23 (and U19,
  U20, U21) FAILED. Restored, 7 passed again.
- refactor: none needed
- commit: the commit that adds this entry
