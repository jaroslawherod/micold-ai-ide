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

## Cycle 5: U24–U27 — one page of open issues, and what is sent for it

- test: `crates/micold-core/tests/github_parse.rs` (new), 4 tests; fixtures
  `tests/fixtures/gh/list_page.json`, `list_page_last.json`, `list_not_found.json` (captured from
  `gh` 2.54.0: a repository that does not exist), `list_rate_limited.json`
- stub: `Issue` (accessors; `row_text` empty), `IssuePage`, `IssueLoadError`, `LIST_QUERY`,
  `parse_list_page` returning `Err(Other(""))`, `list_args` returning `[]`
- red: `scripts/build-lock.sh cargo test -p micold-core --test github_parse`
  -> `test result: FAILED. 0 passed; 4 failed`. Decisive lines: `a_list_page_parses`: `a
  well-formed page: Other("")`; `graphql_errors_are_classified`: `an unresolvable repository is one
  the sign-in cannot see / left: Other("") / right: NoAccess`; `list_args_send_only_the_repository`:
  `left: [] / right: ["api", "graphql", "--hostname", "github.com", "-f", "query=…", "-f",
  "owner=o", "-f", "name=r"]`; `row_text_shows_labels_only_when_present`: `called
  Result::unwrap() on an Err value: Other("")`.
- green: `parse_list_page` over `serde_json::Value` (first `errors[]` entry: `NOT_FOUND` ->
  `NoAccess`, `RATE_LIMITED` -> `RateLimited`, other -> `Other(message)`; `hasNextPage: false` ->
  no cursor; labels capped at 20), `Issue::new` deriving `row_text`, `list_args` with `-f` for
  every variable. -> `4 passed; 0 failed`; `mise run test-core` passed, 0 failed
- refactor: none needed
- commit: the commit that adds this entry

## Cycle 6: U28–U30 — paging up to the 1,000-issue cap

- test: `crates/micold-core/tests/github_load.rs` (new), 3 tests with `FakeIssueSource`
- stub: `ISSUE_LOAD_CAP`, `IssueListing`, `trait IssueSource { list_open }`, `load_listing`
  returning `Err(Other(""))`. `FakeIssueSource` (scripted pages/errors, records calls) is the test
  double, written in full with the stub.
- red: `scripts/build-lock.sh cargo test -p micold-core --test github_load`
  -> `test result: FAILED. 0 passed; 3 failed`. Decisive lines: `pages_concatenate_until_the_last`:
  `loaded: Other("")`; `the_cap_is_1000`: `called Result::unwrap() on an Err value: Other("")`;
  `first_error_aborts_and_empty_is_complete`: `a failed page fails the whole load; … / left:
  Other("") / right: Offline`.
- green: `load_listing` pages with the previous page's cursor, stops at no cursor or at the cap,
  truncates to the cap, returns the first error, and sets `complete = held >= total_open`.
  -> `3 passed; 0 failed`; `mise run test-core` passed, 0 failed
- refactor: none needed
- commit: the commit that adds this entry

## Cycle 7: U31–U35 — classifying gh's failures and wording them

- test: `crates/micold-core/tests/github_classify.rs` (new), 5 tests over the stderr fixtures in
  `tests/fixtures/gh/` (five captured from gh 2.54.0, the rest from gh's documented texts)
- stub: `classify` returning `Other("")`, `IssueLoadError::message` returning `""`
- red: `scripts/build-lock.sh cargo test -p micold-core --test github_classify`
  -> `test result: FAILED. 0 passed; 5 failed`. Decisive lines: `not_signed_in`: `left: Other("")
  / right: NotSignedIn`; `no_access`: `left: Other("") / right: NoAccess`;
  `offline_rate_limited_timed_out`: `left: Other("") / right: Offline`; `unknown_text_is_other`:
  `left: Other("") / right: Other("something unexpected happened")`;
  `messages_name_cause_and_remedy`: `ToolMissing / left: "" / right: "Couldn't read issues: the
  GitHub CLI (`gh`) isn't installed. …"`.
- green: `classify` per R8 (timeout, spawn failure, exit 4 / auth text, rate limit before the
  403 access texts, offline texts, else the first non-empty stderr line or the exit status);
  `message` returns the §5 texts. -> `5 passed; 0 failed`; `mise run test-core` 1300 passed,
  0 failed
- refactor: none needed
- notes: every test was red on the stub, so no deliberate-mutant check was needed
- commit: the commit that adds this entry

## Cycle 8: U36–U40 — a worktree name from an issue title

- test: `crates/micold-core/tests/naming_from_title.rs` (new), 5 tests
- stub: `ISSUE_NAME_SLUG_MAX = 0`, `name_from_title` returning `""`
- red: `scripts/build-lock.sh cargo test -p micold-core --test naming_from_title`
  -> `test result: FAILED. 2 passed; 3 failed`. Decisive lines: `fits_and_the_50_boundary`:
  `left: 0 / right: 50`; `cut_at_word_boundary`: `left: "" / right: "When the sidebar is collapsed
  the create worktree"`; `a_long_first_word_is_cut_at_50`: `left: "" / right: "aaaa…"` (50).
  `empty_slug_yields_empty_name` and `slug_never_exceeds_50` passed on the stub.
- green: `name_from_title` normalises whitespace, keeps a title whose slug fits, else the longest
  whole-word prefix with a non-empty slug that fits, else the slug's first 50 characters. The
  first green attempt returned `"🔥🔥 !!!"` for a title that slugs to nothing
  (`empty_slug_yields_empty_name`: `left: "🔥🔥 !!!" / right: ""`), so that test was red
  against a real implementation; an empty-slug check made it pass. -> `5 passed; 0 failed`;
  `mise run test-core` passed, 0 failed
- mutant: `slug_never_exceeds_50` with the fit bound mutated to `ISSUE_NAME_SLUG_MAX + 1` ->
  `"xxx…x y" -> "xxx…x y" slugs past 50`, FAILED; code restored exactly
- refactor: none needed
- commit: the commit that adds this entry

## Cycle 9: U41–U43 — `RemoteList` on the wire, protocol 16

- test: `crates/micold-daemon/tests/remote_list.rs` (new, 2 tests over `serve_connection`);
  `protocol_roundtrip.rs` gains a `ClientMsg::RemoteList` and an `OperationResult::RemoteList`
  sample; `schema_hash.rs` pins `PROTOCOL_VERSION` to 16
- stub: the two variants in `messages.rs`, and a daemon arm answering
  `OperationError { kind: Internal, message: "" }` so the workspace builds
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test remote_list`
  -> `test result: FAILED. 0 passed; 2 failed`. Decisive lines: `a_repository_answers_its_remotes`:
  `expected a RemoteList result, got OperationError { req: 7, kind: Internal, message: "", detail:
  None }`; `a_non_repository_is_rejected`: `left: (Internal, "", None) / right: (Refused, "project
  is not a git repository", None)`.
  `scripts/build-lock.sh cargo test -p micold-core --test schema_hash --test protocol_roundtrip`
  -> `the_wire_changes_for_this_feature_cost_exactly_one_version_bump`: `left: 15 / right: 16`;
  the round-trip samples passed on the stub (derived serde).
- green: the daemon arm (non-repository -> `reject_non_repo`; `spawn_blocking` `remote_list` +
  `parse_remote_list`; git failure -> `GitFailed "could not list remotes"`; join failure ->
  `Internal`), `PROTOCOL_VERSION` 15 -> 16 with its doc line. -> `2 passed`, `9 passed`, `8
  passed`; `cargo check --workspace --all-targets` clean; `mise run test-core` passed, 0 failed
- mutant: `#[serde(skip)]` on `OperationResult::RemoteList::remotes` ->
  `every_daemon_message_json_round_trips` FAILED; code restored exactly
- refactor: none needed
- commit: the commit that adds this entry
- notes (added after the cycle 9 entry, same commit): the first `mise run test-core` after green
  failed `protocol_auth.rs::the_protocol_version_is_fifteen` (`left: 16 / right: 15`), a second
  version pin. Every earlier bump updated it in place (its doc lists each), so it follows: renamed
  `the_protocol_version_is_sixteen`, asserting 16, with a doc sentence for 034. Re-run: `mise run
  test-core` 1305 passed, 0 failed.

## Cycle 10: U94 — `GhCli`, the production issue source

- test: `crates/micold-core/tests/github_gh_cli.rs` (new), 3 tests against a stub `gh` the test
  writes (`sh` on Unix, `.cmd` on Windows) that records its arguments, environment and working
  directory
- stub: `GhCli { gh, timeout }` with `new` / `with_timeout`, `list_open` returning `Err(Other(""))`
- red: `scripts/build-lock.sh cargo test -p micold-core --test github_gh_cli`
  -> `test result: FAILED. 0 passed; 3 failed`. Decisive lines: `gh_cli_runs_gh_as_specified`:
  `the stub's page parses: Other("")`; `a_hung_gh_is_timed_out`: `left: Other("") / right:
  TimedOut`; `exit_4_is_not_signed_in`: `left: Other("") / right: NotSignedIn`.
- green: `list_open` runs `gh` under `no_window` with exactly `list_args`, the five variables,
  stdin null, cwd = the user's home, through `run_bounded` (10 s default); exit 0 parses the page,
  anything else goes to `classify`. -> `3 passed; 0 failed`; `mise run test-core` 1308 passed,
  0 failed (includes `background_spawns_hide_console.rs`)
- refactor: removed a first-draft branch that parsed stdout on a non-zero exit carrying
  `errors[]`. No test drove it, and contracts/github-issue-source.md §3 sends a list failure to
  `classify` (the stdout rule there is for search, a later milestone). Suite re-run green.
- notes: T015 also carries `[U63]` (`GhCli` named only in `Capabilities::real()`), a client gate
  whose seam arrives with the client wiring in M2. T015's implementation is complete in M1 scope,
  so it is ticked; U63 stays TODO for M2.
- commit: the commit that adds this entry

## Cycle 11: U98–U102 — Review A (code-review, high) findings

- tests (each extends its file): `github_locate.rs::relative_path_entries_are_dropped_and_quotes_removed`,
  `github_remote.rs::managed_user_owner_with_underscore`,
  `github_load.rs::a_page_that_makes_no_progress_ends_the_load`, `github_classify.rs::no_access`
  (new fixture `graphql_forbidden.stderr`, from GitHub's documented text), and a `GH_DEBUG`
  assertion in `github_gh_cli.rs::gh_cli_runs_gh_as_specified`
- red: `scripts/build-lock.sh cargo test --no-fail-fast -p micold-core --test github_locate --test
  github_remote --test github_load --test github_gh_cli` and `--test github_classify`. Decisive
  lines: `only absolute directories are candidates: ["bin", "/from/profile", ".",
  "node_modules/.bin", …]`; `left: None / right: Some("jdoe_acme/tool")`; `a stall is not an
  error: Other("FakeIssueSource: no page scripted for this call")`; `left: Other("GraphQL: Resource
  not accessible by personal access token (repository.issues)") / right: NoAccess`; `GH_DEBUG is
  removed, so debug traces never reach the stderr `classify` reads: GH_PROMPT_DISABLED=1 …`.
- green: `HostOs::is_absolute` (text rule per OS) filters candidates, and entries are unquoted;
  `_` allowed in owners (data-model.md and T012 updated to match); `load_listing` stops on an
  empty page or a repeated cursor; `resource not accessible` classified `NoAccess`;
  `env_remove("GH_DEBUG")`. -> all six affected files pass.
- also (no test, structural): `run_bounded` no longer returns early when `try_wait` fails; it
  kills the group and joins both readers first, then reports `SpawnFailed`. `kill_process_group`
  is private again, as it was in `env_include.rs`. A failing `try_wait` cannot be induced from a
  test.
- refactor: none needed
- commit: the commit that adds this entry
- notes (Review B F1): the macOS cross-check that T010 asks for after the structural move has run
  on the full branch: `scripts/build-lock.sh cargo check --workspace --target aarch64-apple-darwin`
  exit 0 (after `mise run gate` passed at 939ef459). Review B also ran `cargo check -p micold-core
  --all-targets` for `aarch64-apple-darwin` and `x86_64-pc-windows-msvc`: both clean.

## Review round 2 (sonnet): three MINOR fixes, no new behavior

- `run_bounded` reaps the child (`child.wait()`) after the kill on the failed-`try_wait` path.
- `PATH` entries are unquoted on Windows only; on Unix `"` is a file-name character. The Windows
  and Unix cases in `relative_path_entries_are_dropped_and_quotes_removed` still hold.
- research.md R3 and R8, data-model.md and contracts/github-issue-source.md §3 now describe the
  relative-entry rule, the "Resource not accessible" `NoAccess` text and the `GH_DEBUG` removal.

## M2 baseline

- `cargo test --workspace` on 454c716c (origin/main after PR #460) before any M2 test: exit 0.

## Cycle 12: M2 reds, batched per test file (U96, U44–U65, U75, A1–A9)

- deviation (as M1): the reds were written per file and run together against minimal stubs,
  not one behavior at a time. The stubs declared the types, messages and capability shapes so the
  tests compiled; every reducer arm was a no-op, the three readers returned `None`, `press()`
  always returned the press, and `Capabilities::real()` built no `GhCli`.
- test fixes made before the first red run, no implementation present: the expected branch in
  `issue_source_previews_as_new` was corrected to `fix/42_crash-…` (`naming::derive` joins the
  ticket with `_`); `Direction::Down` was corrected to `Direction::Next` (the enum's real name);
  the U65 builder test reads the declaration from source, because `ui::material` is private to
  the binary and cannot be called from an integration test.
- red, `scripts/build-lock.sh cargo test -p micold-client --test issue_source_state`
  -> `test result: FAILED. 0 passed; 15 failed`. Decisive lines: `issue_source_state.rs:102:31`
  (`choosing an available source starts a load`), `:150:5` (captions), `:193:5` (availability),
  `:229:5` (source chosen only when available).
- red, `--test issues_are_requested_only_on_named_events` -> `1 passed; 1 failed`:
  `every_allowlist_entry_still_matches` (`these ALLOWED entries match nothing`).
- red, `--test showcase_completeness` -> `the_toggle_chip_is_posed_disabled` FAILED at `:768:5`
  (`the disabled chip is missing`).
- red, `--test no_concrete_implementations` -> `each_implementation_is_chosen_in_exactly_one_place`
  FAILED at `:279:5` (`GhCli` chosen 0 times) (U63).
- red, `--lib ui::material::toggle_chip` -> `a_disabled_chip_emits_no_press` FAILED:
  `left: Some(7) / right: None` (U64).
- red, `--bin micold-ai-ide issue_source` -> `0 passed; 13 failed`. Decisive lines:
  `main_tests.rs:4199:38` (`opening the form asks for the remotes`), `:4231:9` and `:4286:9`
  (the remotes' error and disconnected captions).
- passed at first run, pinned by a deliberate mutant:
  `material_builder_api.rs::a_toggle_chip_can_be_disabled_in_the_chain` passed against the stub's
  declaration. A mutant (`disabled(&mut self, …)`, not chainable) could not be observed through
  it: the library stops compiling at the call sites that chain it, which is the stronger check.
  U65's observable half is the showcase pose red above. U75,
  `scripts/build-lock.sh cargo test -p micold-core --release --test typeahead_budget`, passed
  (ranking 1,000 rows well under 50 ms); with `ISSUE_BUDGET_MS` set to 0 it failed with
  `ranking 1,000 issue rows for "cra" took 0.65ms, over SC-003's 0ms`, then was restored.
- green: the reducer arms in `features/worktree_form.rs` (availability from `choose_remote`, seq
  handed out on an accepted choice or retry and kept outside the form, only the awaited result
  applies, local ranking over `row_text`, a pick fills ticket and `name_from_title`, `preview()`
  treats `Issue` as `New`); `PendingOp::RemoteList` and its reply, error and disconnect arms;
  `shell/issues.rs` (remotes on open, the load on `spawn_blocking` with the env-include snapshot
  resolved on a miss and cached from `IssuesLoaded`, `ToolMissing` without building a source);
  `IssueTooling` in `Capabilities` with `GhCli` named once in `real()`, and a test default that
  never finds `gh`; `ToggleChip::disabled` using `on_press_maybe` and the disabled tokens.
  -> each target above passes (15, 2, 11, 35, 14, 2, 13); `mise run gate` exit 0.
- refactor: `naming_inputs` shared by the New and Issue sources in `ui/worktree_form.rs`; the two
  capability closure types named (`LocateGh`, `IssueSourceFactory`) for clippy's
  `type_complexity`. Gate re-run green.
- notes: the caption under the source switch moved the Type select from child 2 to child 3 of the
  dialog's fields, so the `add-worktree-dialog-type-menu-open` press path and anchor moved with it,
  and `layout_snapshot.txt` was regenerated with four new covered states (T030).
- commit: the commit that adds this entry

## Cycle 13: U103–U106 — M2 review round 1 (code-review A, conformance B)

- red: `scripts/build-lock.sh cargo test -p micold-client --test issue_source_state` against a
  stub `issue_cap_caption` returning `None` -> 15 passed, 2 failed:
  `issue_source_state.rs:557:5 left: Some(42) right: None` (U103) and
  `issue_source_state.rs:584:5 left: None right: Some("Showing the 3 most recently updated of 1,234 open issues.")`
  (U104). `--bin micold-ai-ide issue_source` -> 14 passed, 1 failed:
  `main_tests.rs:4478:9 left: Checking right: Unavailable("Couldn't read this repository's remotes: no project is open")`
  (U106).
- test-after, pinned by a mutant: U105 (`issue_a_row_pick_resolves_to_its_issue`) passed at first
  run against the existing `on_issue_row_picked`; A3 now picks through `IssueRowPicked` too. With
  the mutant `issue_number_at(index + 1)` both failed (13 passed, 2 failed), then it was restored.
- green: `reset_issues` clears `picked_issue`; `issue_cap_caption` in the reducer with `thousands`
  moved beside it, and the UI reads it; `on_form_opened` answers `RemotesListed(Err("no project is
  open"))` when no project is active. -> 17 passed, 15 passed, 2 passed.
- also fixed without a new behaviour test (review A): the load's snapshot no longer overwrites a
  newer cache entry (`entry().or_insert`, #1); an unexpected answer to `RemoteList` becomes an
  error instead of leaving `Checking` (#4); rows and the selected row come from `held()` in one
  pass (#6, #7); the snapshot is resolved through `resolve_env_include` (#10).
- commit: the commit that adds this entry

## Cycle 14: U66–U69, U70 (core), U95 — M3 search in the core, reds batched per test file

- baseline: `origin/main` at `a17e6cde` (M2 PR #468 merged, CI green); not re-measured locally
  before the first red. The gate runs the full workspace before the PR.
- red: with stubs that only make the symbols resolve (`search_args` → `[]`, `parse_search` →
  `Ok([])`, `merge_searched` → its input, `search_open` → `Ok([])`):
  `scripts/build-lock.sh cargo test --no-fail-fast -p micold-core --test github_search --test github_parse --test github_gh_cli`
  -> `github_parse` 4 passed, 3 failed:
  `github_parse.rs:155:5 left: [] right: [1200, 1300]` (U66),
  `github_parse.rs:176:5 left: [] right: [4312]` (U67),
  `github_parse.rs:227:5 left: [] right: ["api", "graphql", …, "q=repo:o/r is:issue is:open crash on open"]` (U68);
  `github_search` 1 passed, 1 failed: `github_search.rs:29:5 left: [1200, 42, 1100] right: [1200, 1100]` (U69);
  `github_gh_cli` 3 passed, 1 failed: `github_gh_cli.rs:218:5 left: [] right: [4312]` (U95).
- test-after: U70's core half (`a_body_only_match_is_hidden`) passed at first run — it pins
  feature 021's `typeahead::rank` over `row_text`, which already existed and has its own mutant-
  checked suite; it is recorded as characterization of that rule for searched issues.
- green: `SEARCH_QUERY` / `SEARCH_WITH_NUMBER_QUERY`; `search_args` (`-f q=…` always; `-f owner -f
  name -F n=N` only for `N`/`#N` fitting `i32`); `parse_search` (search nodes then the lookup, open
  only, first of each number; a sole NOT_FOUND at `["repository","issue"]` forgiven, any other entry
  classified by the same `graphql_error_of` the list uses); `merge_searched`; `IssueSource::search_open`
  on `FakeIssueSource` (scripted `with_search`, recorded `search_calls`) and `GhCli`.
  -> 4, 7, 2 and `github_load` 4 passed.
- refactor: `GhCli::run` shared by `list_open` and `search_open`, holding the partial-response rule
  (stdout read whenever it is JSON with `data`, whatever the exit status). This also answers M1's
  declined conformance B F2: `parse_list_page`'s `errors[]` branch is now reached for a real `gh`,
  which exits 1 on any GraphQL error — pinned by the `ListNotFound` stub in
  `a_partial_response_is_parsed` (exit 1, empty stderr, `list_not_found.json` on stdout → `NoAccess`;
  before, `classify` read the empty stderr as `Other("gh exited with status 1")`).
- commit: the commit that adds this entry

## Cycle 15: U70 (reducer), U71–U74, U62, A10 — the search in the form and the shell

- red (reducer): with `SearchState::{Pending, Searching, Failed}` and the two messages declared but
  handled as no-ops, and `issue_search_status` → `None`:
  `scripts/build-lock.sh cargo test --no-fail-fast -p micold-client --test issue_source_state` ->
  16 passed, 5 failed: `search_only_when_incomplete`, `a_newer_keystroke_discards_an_older_search`,
  `a_failed_search_keeps_loaded_matches`, `an_unmatched_searched_issue_is_hidden` each at
  `issue_source_state.rs:627:18` "no search pending: Idle" (U71–U73, U70); and
  `the_cap_caption_counts_the_loaded_and_the_open` `:588:5 left: Some("Showing the 3 most recently
  updated of 1,234 open issues.") right: Some("… — search also looks on GitHub.")` (T040's caption,
  a stated test change: contracts/issue-picker-ui.md §2 words the caption that way).
- red (shell), with only `ISSUE_SEARCH_DEBOUNCE` declared:
  `--test issues_are_requested_only_on_named_events` 1 passed, 1 failed:
  `every_allowlist_entry_still_matches` "these ALLOWED entries match nothing" (the four
  `start_issue_search` lines, U62);
  `--bin micold-ai-ide issue_search` 0 passed, 2 failed: `issue_search_is_debounced`
  `main_tests.rs:4897:9` "the search waits out the debounce" (U74);
  `issue_search_finds_an_issue_beyond_the_cap` `main_tests.rs:4954:9 left: [] right: [("o/r", "1100")]` (A10).
- green: reducer — a keystroke on a capped list with text hands out a fresh seq as
  `Pending{seq}`, otherwise `Idle`; `IssueSearchDue` moves only the current `Pending` to
  `Searching`; `IssueSearched` applies only to the current `Searching`, replaces `searched` with
  `merge_searched` and re-ranks, or becomes `Failed`; `IssueRetry` from a failed search →
  `Searching{new seq}`; `issue_search_status`; State helpers `pending_issue_search`,
  `awaited_issue_search`, `issue_search_request`. Shell — `on_issue_query_changed` (300 ms timer →
  `IssueSearchDue`), `on_issue_search_due`, `retry_issue_search`, `start_issue_search`
  (`spawn_blocking(search_open)` on a source built from the load's `gh`, never locating again).
  UI — "Searching GitHub…" / the failure + **Retry** under the picker.
  -> `issue_source_state` 21 passed; the gate test 2 passed; `--bin micold-ai-ide issue` 17 passed.
- notes: A10's first run failed on the wrong assertion — the fixture assumed `1100` matches only
  #17 locally, but 021's approximate tier also offers #1000; the assertion was corrected to
  "#17 shown, #1100 not yet" before any implementation, and the body-only issue renumbered to
  #5555 for the same reason. The timer is built inside the future (`tokio::time::sleep` panics
  outside a runtime); the retry arm was split into `retry_issue_search` so the gate's existing
  load-retry line stayed exact.
- commit: the commit that adds this entry

## Cycle 16: U107–U111 — M3 review A (code-review, high) findings

- red: `scripts/build-lock.sh cargo test --no-fail-fast -p micold-client --test issue_source_state`
  -> 21 passed, 4 failed:
  `a_search_does_not_start_while_creating` `:869:5 left: Searching { seq: 2 } right: Pending { seq: 2 }` (U110, #7);
  `a_search_answer_keeps_the_highlighted_issue` `:817:5 left: Some(5) right: Some(999999)` (U107, #1);
  `clearing_the_query_forgets_searched_issues` `:838:5 left: [7, 42, 108, 1200] right: [7, 42, 108]` (U108, #4);
  `whitespace_alone_does_not_search_again` `:854:5` (search `Pending`, not `Idle`) (U109, #6).
- test-after, pinned by a mutant: U111 (#2). The fix and the `SamlRefused` stub were written
  together; with the mutant `parse(stdout)` in place of `parse(stdout).map_err(|_| classify(&outcome))`
  `github_gh_cli` failed 2 of 4 (`a_partial_response_is_parsed` `:248:5 left: Other("Resource
  protected by organization SAML enforcement.") right: NoAccess`, and `exit_4_is_not_signed_in`),
  then it was restored and passed 4 of 4.
- green: `issue_search_due` runs under `while_editing_unprompted`; `issue_searched` re-seats the
  highlight on the issue it was on; clearing the query clears `searched`; a whitespace-only change
  keeps the search state; `GhCli::run` parses a non-zero exit's stdout and, when the parser refuses
  it, classifies from stderr — `holds_graphql_data` and its second JSON parse are gone.
  -> `issue_source_state` 25 passed; `github_gh_cli` 4, `github_parse` 7, `github_search` 2,
  `github_load` 4; `--bin micold-ai-ide issue` 17 passed.
- notes: this supersedes cycle 14's note on M1 conformance B F2. The `ListNotFound` stub (exit 1,
  empty stderr) is gone: real `gh` repeats the GraphQL message on stderr, and stderr names what
  GraphQL's error types do not (SAML, scopes), so a refused list answer stays with `classify`, and
  `parse_list_page`'s `errors[]` branch remains the exit-0 path M1 declined.
- commit: the commit that adds this entry

## Cycle 17: U110 (revised), U111 — M3 review A round 2

- red: U110 rewritten to the opposite expectation (a debounce ending while a create runs still
  searches; round 1's #7 fix left the search `Pending` for good once the form returned to editing):
  `scripts/build-lock.sh cargo test -p micold-client --test issue_source_state creating` ->
  `a_search_due_while_creating_is_not_left_pending` `:874:5 left: Pending { seq: 2 } right: Searching { seq: 2 }`.
  U111 extended with the `ListNotFound` stub (exit 1, empty stderr, NOT_FOUND answer):
  `scripts/build-lock.sh cargo test -p micold-core --test github_gh_cli partial` ->
  `github_gh_cli.rs:259:5 left: Other("gh exited with status 1") right: NoAccess`.
- green: `issue_search_due` back on `with_form`; `GhCli::run` keeps an error the parser types
  exactly and sends only `Other` to `classify`; the highlight falls back to `rematch_issues`'
  clamp when the answer dropped its issue (no new test: MINOR); the redundant `listing.complete`
  arm folded; module doc corrected. -> `github_gh_cli` 4, `github_parse` 7, `issue_source_state`
  25, `--bin micold-ai-ide issue` 17 passed.
- notes: U110's first form (cycle 16) is superseded, with the reason above; the log keeps both.
- commit: the commit that adds this entry
