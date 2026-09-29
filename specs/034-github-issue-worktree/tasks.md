---
description: "Task list for feature 034 — create a worktree from a GitHub issue"
---

# Tasks: Create a Worktree from a GitHub Issue

**Input**: Design documents from `/specs/034-github-issue-worktree/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY and must be observed failing before their implementation. Behavior ids (`[A#]`,
`[U#]`) refer to [tdd/test-list.md](./tdd/test-list.md); `/speckit.tdd.run` ticks a task from them. Every phase lists its failing
tests first. The GUI exception is claimed only for `src/ui/` composition, verified by the recorded
quickstart §B pass.

**Documentation**: Per Constitution Principle VII, each user-facing story carries its own user-guide
task in the milestone that ships it (CI's user-guide gate).

**Cross-platform**: Per Constitution Principle VI, all platform-sensitive decisions (`locate_gh`,
`run_bounded`, `choose_remote`, `classify`) are in `micold-core`, whose suite CI runs on Linux,
macOS and Windows. The only `cfg` arms touched are `run_bounded`'s existing kill arms, moved with it.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story the task belongs to (US1–US3)
- Every description carries an exact file path

## Path Conventions

Three-crate workspace: `crates/micold-core/` (render-free rules), `crates/micold-daemon/` (one new
read-only RPC arm), `crates/micold-client/` (form, settings, shell, UI). Build and test through
`mise run <task>` (CLAUDE.md). Shell event tests live in `crates/micold-client/src/main_tests.rs`
(the binary's `tests` module); name each new shell test with `issue` so a milestone's Verify filter
selects it.

---

## Phase 1: Setup (Shared Infrastructure)

No setup tasks: no new crate, dependency, binary or configuration (plan, Structure Decision).

---

## Phase 2: Foundational (Blocking Prerequisites)

No foundational tasks. Every prerequisite (the bounded runner move, remote discovery, the issue
source) serves User Story 1 only and ships in its first slice below; US2 and US3 build on US1.

---

## Phase 3: User Story 1, slice A — the issue source and remote discovery (Priority: P1) 🎯 MVP

**Goal**: Everything US1 decides, render-free and tested: the bounded runner with pipe draining,
locating `gh`, the GitHub repository behind a remote, the `RemoteList` RPC (protocol 16), loading
and classifying open issues through `gh`, and the name derived from a title. No UI reaches it yet
(milestones.md rule 6); slice B wires it.

**Independent Test**: `mise run test-core` and `scripts/build-lock.sh cargo test -p micold-daemon --test remote_list` pass; the daemon answers `RemoteList` for a repository with a GitHub remote.

### Tests for User Story 1, slice A (MANDATORY — Constitution Principle I) ⚠️

- [X] T001 [P] [US1] [U1] [U2] [U3] [U4] [U5] Write `crates/micold-core/tests/process_run_bounded.rs` against `micold_core::process::run_bounded` (research R6):
  - A child that sleeps past the bound returns `RunOutcome::TimedOut` within bound + 1 s and leaves no child process.
  - A child writing 1 MiB to stdout and 64 KiB to stderr returns `Exited` with every byte of both (pipes drained while waiting).
  - A child exiting non-zero returns its status, stdout and stderr.
  - The existing `crates/micold-core/tests/env_include*.rs` suites stay green unchanged.
- [X] T002 [P] [US1] [U11] [U12] [U13] [U14] [U15] [U16] Write `crates/micold-core/tests/github_remote.rs` (contracts/remote-list-rpc.md §4, research R5):
  - `GithubRepo::from_remote_url` accepts `https://github.com/o/r(.git)(/)`, `http://`, `git@github.com:o/r.git`, `ssh://git@github.com/o/r.git`, `ssh://git@ssh.github.com:443/o/r.git`, `git://github.com/o/r`, `GITHUB.COM`, and userinfo forms `https://user@github.com/o/r` and `https://x-access-token:T@github.com/o/r`; the token never appears in the value or its `Display` (`owner/name`).
  - It rejects `www.github.com`, `github.example.com`, GitLab/Bitbucket URLs, local paths and an alias `gh:o/r`.
  - `choose_remote`: `origin` on GitHub wins; origin on GitLab + upstream on GitHub picks upstream; two GitHub remotes without origin picks the first in config order; none → `NoGithubRemote`.
- [X] T003 [P] [US1] [U6] [U7] [U8] [U9] [U10] Write `crates/micold-core/tests/git_remotes.rs` (contracts/remote-list-rpc.md §1):
  - `parse_remote_list`: `remote.<name>.url <url>` lines, a dotted remote name (`remote.a.b.url`), a duplicate `url` for one remote (first wins), empty output → empty, config order kept.
  - `GitCli::remote_list` on a temp repo with two remotes lists both; on a repo with none returns `Ok("")` (git's exit 1 mapped).
  - A global `insteadOf` rewrite (`GIT_CONFIG_GLOBAL` pointed at a temp file) is **not** applied to the listed URL.
- [X] T004 [P] [US1] [U17] [U18] [U19] [U20] [U21] [U22] [U23] Write `crates/micold-core/tests/github_locate.rs` against a fake `exists` probe (contracts/github-issue-source.md §1):
  - Each `HostOs` value's well-known table, `path_separator()` (`:`/`;`) and `exe_name()` (`gh`/`gh.exe`), on every host.
  - Env-include `PATH` entries come before process `PATH`, which comes before well-known dirs; duplicates keep first position; empty components dropped.
  - `MacOs` with the Dock `PATH` `/usr/bin:/bin:/usr/sbin:/sbin` finds `gh` only in `/opt/homebrew/bin`; `Windows` finds `gh.exe` only under `%LOCALAPPDATA%\Microsoft\WinGet\Links`; a `Path` (not `PATH`) key from the env-include snapshot is honoured; nothing found → `None`.
- [ ] T005 [P] [US1] [U24] [U25] [U26] [U27] Write `crates/micold-core/tests/github_parse.rs` for the list page, with JSON fixtures under `crates/micold-core/tests/fixtures/gh/` (contracts/github-issue-source.md §3–4):
  - `parse_list_page` maps `number`, `title`, `updatedAt`, up to 20 label names, `totalCount` and `pageInfo` into `IssuePage`; `Issue::row_text` is `#<number> <title>` plus `  ·  <l1>, <l2>` only when labelled.
  - `errors[]` with `type: NOT_FOUND` → `NoAccess`; `RATE_LIMITED` → `RateLimited`; malformed JSON → `Other`.
  - `list_args(repo, cursor)` (pure argument builder) is `api graphql --hostname github.com -f query=… -f owner=… -f name=…` plus `-f cursor=…` only with a cursor; every string uses `-f`; a repository named `1` or `true` stays a string; no path, branch or file name appears (FR-025).
- [ ] T006 [P] [US1] [U28] [U29] [U30] Write `crates/micold-core/tests/github_load.rs` with `FakeIssueSource` (contracts/github-issue-source.md §4):
  - Pages concatenate in order until `next_cursor` is `None`; `complete` is true when held ≥ `total_open`.
  - At `ISSUE_LOAD_CAP` (1,000) paging stops, the last page is truncated to the cap, and `complete` is false when `total_open` > 1,000.
  - The first error aborts the whole load and is returned; zero open issues → an empty, complete listing.
- [ ] T007 [P] [US1] [U31] [U32] [U33] [U34] [U35] Write `crates/micold-core/tests/github_classify.rs` with captured `gh` output fixtures `crates/micold-core/tests/fixtures/gh/*.stderr` (research R8):
  - `classify(&process::RunOutcome)` yields `NotSignedIn` (exit 4, "gh auth login", HTTP 401), `NoAccess` (404, 403, SAML), `Offline` (DNS/connection errors), `RateLimited`, `TimedOut` (the `TimedOut` outcome), and `Other(first non-empty stderr line)` for unknown text.
  - `IssueLoadError::message(&repo)` returns exactly the texts of contracts/github-issue-source.md §5, including `ToolMissing`, and names `owner/name` for `NoAccess`.
- [ ] T008 [P] [US1] [U36] [U37] [U38] [U39] [U40] Write `crates/micold-core/tests/naming_from_title.rs` (contracts/issue-naming-and-typing.md §1): every row of the §1 table (fits; longest whole-word prefix with slug ≤ 50; one 70-character first word → first 50 characters of the slug; emoji/punctuation-only → `""`; whitespace normalised), and the property `slugify(&name_from_title(t)).len() <= 50` over a corpus.
- [ ] T009 [US1] [U41] [U42] [U43] Write the `RemoteList` wire tests (contracts/remote-list-rpc.md §2):
  - `crates/micold-daemon/tests/remote_list.rs`: a temp repo with `origin` on GitHub and `upstream` elsewhere answers `OperationResult::RemoteList` with both, in config order; a non-repository project is rejected exactly as `BranchList` is (`reject_non_repo`).
  - `crates/micold-core/tests/protocol_roundtrip.rs` covers `ClientMsg::RemoteList` and `OperationResult::RemoteList`.
  - `crates/micold-core/tests/schema_hash.rs` pins `PROTOCOL_VERSION` to 16.
- [ ] T093 [US1] [U94] Write `crates/micold-core/tests/github_gh_cli.rs`: a stub `gh` written by the test (a `sh` script on Unix, a `.cmd` on Windows) records its arguments, environment and working directory and prints a fixture page. `GhCli::list_open` passes exactly `list_args`, sets `GH_PROMPT_DISABLED=1`, `GH_NO_UPDATE_NOTIFIER=1`, `NO_COLOR=1`, `CLICOLOR=0`, `GH_PAGER=` (empty), runs in the user's home, and parses the page; a stub that sleeps past a short bound (`GhCli::with_timeout`, test-only) yields `TimedOut`; a stub exiting 4 with the not-logged-in fixture yields `NotSignedIn`.

### Implementation for User Story 1, slice A

- [X] T010 [US1] [U1] [U2] [U3] [U4] [U5] Move `run_bounded`, `RunOutcome` and `kill_process_group` from `crates/micold-core/src/env_include.rs` to `crates/micold-core/src/process.rs` as `pub` (process.rs imports `crate::win_job::JobHandle`, which stays where it is), and drain stdout and stderr on reader threads while waiting (research R6). `env_include.rs` calls `process::run_bounded`. Then run `scripts/build-lock.sh cargo check --target aarch64-apple-darwin -p micold-core` (moved `cfg` arms) where the target is installed.
- [X] T011 [P] [US1] [U6] [U7] [U8] [U9] [U10] In `crates/micold-core/src/git.rs`, add `Git::remote_list(&self, repo) -> io::Result<String>` (`config --local --get-regexp ^remote\..+\.url$`, exit 1 → `Ok(String::new())`), `GitRemote { name, url }` (serde), `parse_remote_list`, and `FakeGit::with_remote(repo, name, url)` keeping insertion order.
- [X] T012 [US1] [U11] [U12] [U13] [U14] [U15] [U16] Create `crates/micold-core/src/github.rs` and add `pub mod github;` to `crates/micold-core/src/lib.rs`: `GithubRepo` (fields `owner` non-empty `[A-Za-z0-9-]`, `name` non-empty with `.git` and trailing `/` stripped; constructible only by `from_remote_url`, host `github.com` or `ssh.github.com` ignoring case, userinfo discarded; `Display` = `owner/name`), `RemoteChoice` and `choose_remote`.
- [X] T013 [US1] [U17] [U18] [U19] [U20] [U21] [U22] [U23] In `crates/micold-core/src/github.rs`, add `HostOs` (`current()`, `exe_name()`, `path_separator()`), `LocateInputs { os, env_include_path, process_path, home, env, exists }`, `candidate_dirs` and `locate_gh` with the per-OS well-known table of research R3. No host `split_paths` or `PATHEXT`.
- [ ] T014 [US1] [U24] [U25] [U26] [U27] [U28] [U29] [U30] [U31] [U32] [U33] [U34] [U35] In `crates/micold-core/src/github.rs`, add `Issue` (no `Serialize`; `row_text` derived at construction), `IssuePage`, `IssueListing { issues, total_open, complete }`, `ISSUE_LOAD_CAP = 1_000`, `IssueLoadError` with `message`, `parse_list_page`, `list_args`, `load_listing` and `classify(&process::RunOutcome)`.
- [ ] T015 [US1] [U94] [U63] In `crates/micold-core/src/github.rs`, add `trait IssueSource { fn list_open(&self, repo, cursor) -> Result<IssuePage, IssueLoadError>; }`, `GhCli { gh: PathBuf }` (runs `list_args` via `process::run_bounded(cmd, 10 s)`, env `GH_PROMPT_DISABLED=1`, `GH_NO_UPDATE_NOTIFIER=1`, `NO_COLOR=1`, `CLICOLOR=0`, `GH_PAGER=`, `no_window`, cwd = the user's home) and `FakeIssueSource` (scripted pages/errors, records every call). `GhCli::with_timeout` exists for tests; production uses 10 s.
- [ ] T016 [P] [US1] [U36] [U37] [U38] [U39] [U40] In `crates/micold-core/src/naming.rs`, add `ISSUE_NAME_SLUG_MAX = 50` and `name_from_title` (research R11).
- [ ] T017 [US1] [U41] [U42] [U43] Add `ClientMsg::RemoteList { req, project }` and `OperationResult::RemoteList { remotes }` in `crates/micold-core/src/protocol/messages.rs`, bump `PROTOCOL_VERSION` 15 → 16 with a doc line in `crates/micold-core/src/protocol/version.rs`, and add the daemon arm in `crates/micold-daemon/src/server.rs` (non-repo → `reject_non_repo`; `spawn_blocking(GitCli::new().remote_list)` → `parse_remote_list`; git failure → `OperationError { kind: GitFailed, message: "could not list remotes" }`). Add a `ClientMsg::RemoteList` arm wherever the client matches exhaustively on `ClientMsg` so the workspace builds.

**Checkpoint**: `mise run test-core` and the daemon test pass; the client builds unchanged in behaviour.

---

## Phase 4: User Story 1, slice B — pick an open issue in the form (Priority: P1)

**Goal**: The form offers **GitHub issue**, disabled with a reason when there is no GitHub remote;
choosing it loads and lists open issues with loading, empty and failure states; typing narrows the
list; picking fills ticket and name; the worktree is created by the new-branch path. The type is
still chosen by hand (US2 adds it).

**Independent Test**: quickstart §B1–B5, B7–B9; automated: `issue_source_state.rs`, the shell tests named `issue`, the named-events gate.

### Tests for User Story 1, slice B (MANDATORY — Constitution Principle I) ⚠️

- [ ] T018 [P] [US1] [U96] [U44] [U45] [U46] [U47] [U48] [U49] [U50] [U51] Write `crates/micold-client/tests/issue_source_state.rs` for availability and loading (data-model.md §5):
  - `Opened` starts the form with `github = Checking` (Issue chip disabled, caption "Checking for a GitHub remote…") and `issues = NotRequested`.
  - `RemotesListed(Ok)` with a GitHub remote → `GithubAvailability::Available(repo)`; without one → `Unavailable("This repository has no GitHub remote.")`; `Err(detail)` → `Unavailable("Couldn't read this repository's remotes: <detail>")` (FR-002).
  - `SourceChanged(Issue)` is refused unless `Available` (invariant 1); accepted → `IssueList::Loading { seq }` with a fresh `seq` from `State::issue_request_seq`.
  - `IssuesLoaded` with the awaited `seq` → `Loaded` or `Failed`; any other `seq` is dropped; with no form open every result is dropped (invariant 2, FR-007a; Edge "retry while a load runs", "form closed while issues load").
  - `IssueRetry` acts only from `Failed` (→ `Loading`, new `seq`) and never starts a first load (invariant 1).
  - Switching the source away keeps `type_`, `ticket`, `name` and returns `issues` to `NotRequested`; a late result is then dropped (invariant 7; Edge "switch mid-load", "switch back after a pick").
  - `source_caption()` returns the Checking text, the Unavailable reason, and the opt-in notice naming `owner/name` while `Available` and the source is not `Issue`; `issue_notice()` names `owner/name` while the source is `Issue` (FR-002, FR-025).
  - A `Loaded` listing with zero issues is the empty state (FR-008); closing the form drops the issues (FR-023).
- [ ] T019 [US1] [U52] [U53] [U54] [U55] [U56] [U57] Add pick and search cases to `crates/micold-client/tests/issue_source_state.rs` (contracts/issue-naming-and-typing.md §4–5):
  - `IssueQueryChanged` re-ranks `issue_matches` over `Issue::row_text` at once: `42` finds `#42`, a label name finds its issue, a title fragment narrows (US1 AS3, FR-005); `issue_highlight` stays `< issue_matches.len()` through `IssueHighlightMoved`, `IssueFocused`, `IssueDismissed` (invariant 3), as the branch picker's.
  - `State::issue_number_at(index)` returns the number at `issue_matches[index]` and `None` out of range.
  - `IssuePicked { number }` sets `ticket` = `number.to_string()` (no `#`), `name` = `name_from_title(title)`, `error` = `None`, `picked_issue`, closes the list; it replaces earlier ticket/name (AS4, AS8, FR-009, FR-010, FR-010a) and leaves `type_` unchanged in this slice. Afterwards the fields are ordinary editable fields (AS5, FR-011).
  - A number the listing does not hold, or a form not `Loaded`, makes `IssuePicked` a no-op.
  - `preview()` and `can_submit()` treat `Issue` exactly as `New`, including the existing type/name validation (AS4, AS6, FR-012).
- [ ] T020 [P] [US1] [U62] Write `crates/micold-client/tests/issues_are_requested_only_on_named_events.rs` over the shell source, in the pattern of `refresh_is_only_on_demand.rs`: the issue-source capability is called only from the load and retry arms (FR-003, FR-024). Slice C extends it with the search-due arm.
- [ ] T021 [P] [US1] [U64] [U65] Add `ToggleChip::disabled(bool)` coverage: a disabled chip emits no `on_press` (test beside `crates/micold-client/src/ui/material/toggle_chip.rs`'s existing tests); `crates/micold-client/tests/material_builder_api.rs` sees the chainable method; `crates/micold-client/tests/showcase_completeness.rs` requires the disabled pose.
- [ ] T022 [US1] [U58] [U59] [U60] [U61] In `crates/micold-client/src/main_tests.rs`, add shell tests named `issue…` (contracts/remote-list-rpc.md §3, contracts/issue-picker-ui.md §3):
  - Opening the form sends one `RemoteList` for the active project; its `OperationOk` reaches the form as `RemotesListed(Ok)`; an `OperationError` as `RemotesListed(Err)`; a reply for a project no longer active is dropped.
  - With no connection, opening the form dispatches `RemotesListed(Err("not connected to the session service"))` and raises no toast; a pending `RemoteList` whose connection drops resolves the same way through `on_disconnected`.
  - `IssueRowPicked(index)` dispatches `IssuePicked` for `issue_number_at(index)` and nothing for `None`.
  - With a fake issue-source factory in `Capabilities`, choosing the source runs one load and `IssuesLoaded` carries its `seq`; a cache miss returns the env-include snapshot, which lands in `App::env_include_cache`; `gh` not located → `Failed(ToolMissing)` with no source constructed.
- [ ] T023 [P] [US1] [U75] Add a 1,000-issue-row case to `crates/micold-core/tests/typeahead_budget.rs`: ranking 1,000 realistic `row_text` strings for a 3-character query takes < 50 ms in a release build (SC-003).

- [ ] T066 [US1] [A1] [A2] [A3] [A4] [A5] [A6] [A7] [A8] [A9] Write the US1 acceptance tests A1–A9 of `tdd/test-list.md` in `crates/micold-client/src/main_tests.rs` (named `issue_…`), driving `update_inner` with test `Capabilities` whose issue-source factory returns a `FakeIssueSource`; confirm each fails before T024.

### Implementation for User Story 1, slice B

- [ ] T024 [US1] [U96] [U44] [U45] [U46] [U47] [U48] [U49] [U50] [U51] [U52] [U53] [U54] [U55] [U56] [U57] In `crates/micold-client/src/features/worktree_form.rs`, add `BranchSource::Issue`, `GithubAvailability`, `IssueList` (`NotRequested`, `Loading { seq }`, `Failed { error }`, `Loaded { listing, gh, searched, search }`, with `SearchState::Idle` only until slice C), the `WorktreeForm` fields of data-model.md §5, `State::issue_request_seq: u64` (outside the form, never reset), the messages `RemotesListed`, `IssuesLoaded`, `IssueRetry`, `IssueQueryChanged`, `IssueFocused`, `IssueHighlightMoved`, `IssueDismissed`, `IssueRowPicked`, `IssuePicked { number }`, `State::issue_number_at`, the pure readers `WorktreeForm::source_caption() -> Option<String>` (Checking / Unavailable reason / the Available opt-in notice "GitHub issue reads open issues of owner/name from GitHub." while the source is not `Issue`; `None` when it is) and `WorktreeForm::issue_notice() -> Option<String>` ("Reads open issues of owner/name from GitHub." while the source is `Issue`), and the reducer arms satisfying invariants 1, 2, 3 and 7. Ranking uses `typeahead::rank` over `row_text`. `preview()`/`can_submit()` treat `Issue` as `New`.
- [ ] T025 [US1] [U63] In `crates/micold-client/src/shell/capabilities.rs`, add the issue-source factory `Arc<dyn Fn(PathBuf) -> Arc<dyn IssueSource + Send + Sync>>`; `GhCli` is named only in `Capabilities::real()` (`no_concrete_implementations.rs` stays green); test capabilities get a `FakeIssueSource` factory.
- [ ] T026 [US1] [U58] [U59] In `crates/micold-client/src/shell/daemon_sync.rs`, add `PendingOp::RemoteList { project }` ("read the repository's remotes"), send it on the form's `Opened` for the active project, dispatch `RemotesListed(Err(…))` when not connected (no `send_op` toast), route `OperationOk(RemoteList)` / `OperationError` to the form, drop replies for an inactive project, and add the `on_disconnected` arm.
- [ ] T027 [US1] [U60] [U61] Add the issue load effect in `crates/micold-client/src/shell/daemon_sync.rs` (or a new `shell/issues.rs` registered in `shell/mod.rs`) and route the form messages in `crates/micold-client/src/main.rs`: on an accepted `SourceChanged(Issue)` or `IssueRetry`, `Task::perform(spawn_blocking(…))` locates `gh` (cached env-include snapshot for the project root, else `env_include::snapshot_for` resolved there and returned for caching; process `PATH`; well-known dirs; `HostOs::current()`, `Path::is_file`), reports `ToolMissing` without spawning, else runs `load_listing` through the factory's source → `IssuesLoaded { seq, result: Result<(IssueListing, PathBuf), IssueLoadError> }`. `IssueRowPicked` resolves through `issue_number_at` and dispatches `IssuePicked`.
- [ ] T028 [P] [US1] [U64] [U65] In `crates/micold-client/src/ui/material/toggle_chip.rs`, add `.disabled(bool)` (no `on_press`; Material disabled treatment from existing tokens — 38% content, 12% container), pose it in `crates/micold-client/src/showcase/sections/controls.rs`, and document it in `docs/development/component-library.md`.
- [ ] T029 [US1] [A1] [A2] [A7] [A9] In `crates/micold-client/src/ui/worktree_form.rs`, render the third chip (disabled unless `Available`) and the caption under the switch from `source_caption()` per contracts/issue-picker-ui.md §1 — including the opt-in notice "GitHub issue reads open issues of **owner/name** from GitHub." before the choice (FR-025) — and the issue body per §2: the notice from `issue_notice()`, `StageProgress` "Loading issues from GitHub…", the error + **Retry**, "owner/name has no open issues.", the `Typeahead` wired exactly like the branch picker (`on_pick → IssueRowPicked(index)`), the cap caption when `!listing.complete` — in this slice "Showing the 1,000 most recently updated of N open issues." only; slice C (T040) adds "— search also looks on GitHub." — and the Type/Ticket/Name inputs through a shared private `naming_inputs(form, r, focused)` used by the New source too.
- [ ] T030 [US1] Register the covered states "GitHub chip disabled with reason", "Issue source Loading", "Issue source Failed + Retry" and "Issue source Loaded with rows and the cap caption" in `crates/micold-client/tests/support/covered_states.rs`, and regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` with `UPDATE_LAYOUT_SNAPSHOT=1`; the layout and text-overflow gates pass at 520 px.
- [ ] T031 [US1] Add "From a GitHub issue" to `docs/user-guide/worktrees-and-sessions.md` (under "Creating a worktree"): prerequisites (`gh` installed and `gh auth login`), which remote is used and that aliased (`insteadOf`) remote URLs are not recognised, what is sent to GitHub (FR-025), the 1,000-issue cap, each failure message and its remedy, and that the type is chosen by hand.
- [ ] T067 [US1] [A1] Outer loop green: A1 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T068 [US1] [A2] Outer loop green: A2 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T069 [US1] [A3] Outer loop green: A3 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T070 [US1] [A4] Outer loop green: A4 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T071 [US1] [A5] Outer loop green: A5 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T072 [US1] [A6] Outer loop green: A6 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T073 [US1] [A7] Outer loop green: A7 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T074 [US1] [A8] Outer loop green: A8 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T075 [US1] [A9] Outer loop green: A9 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T032 [US1] Run quickstart §B1–B5 and B7–B9 with the `visual-pass` skill (in §B4 check ticket, name and preview only — the type is chosen by hand until M4, and T052 checks it), time §B2–B5 from the open form to the created worktree (SC-001: under 20 s, no ticket or name typed), and record results, the timing and screenshots in `specs/034-github-issue-worktree/evidence/quickstart-b.md`.

**Checkpoint**: US1 AS1–AS9 work end to end; `mise run gate` passes.

---

## Phase 5: User Story 1, slice C — search beyond the load cap (Priority: P1)

**Goal**: When the loaded list is incomplete, a debounced search also asks GitHub and merges its
matching open issues into the results (US1 AS10, FR-005a).

**Independent Test**: quickstart §B6; automated: `github_search.rs`, the search cases of `issue_source_state.rs`.

### Tests for User Story 1, slice C (MANDATORY — Constitution Principle I) ⚠️

- [ ] T033 [P] [US1] [U66] [U67] [U68] Add to `crates/micold-core/tests/github_parse.rs` the `parse_search` cases with fixtures `search_pr_number.json`, `search_missing_number.json`, `search_closed_number.json` under `crates/micold-core/tests/fixtures/gh/` (contracts/github-issue-source.md §3–4): union of `search.nodes` and `repository.issue`, open only, deduped by number; a sole `NOT_FOUND` at `["repository","issue"]` keeps the hits and raises no error; any other `errors[]` entry is classified. `search_args(repo, text)` uses SEARCH_QUERY with `-f q=repo:<owner>/<name> is:issue is:open <text>` for any text, and SEARCH_WITH_NUMBER_QUERY with `-f owner -f name -F n=N` only when the text is `N` or `#N` fitting `Int`.
- [ ] T034 [P] [US1] [U69] [U70] Write `crates/micold-core/tests/github_search.rs`: `merge_searched` drops numbers already loaded (invariant 4); a searched issue matched by GitHub only in its body is not shown after ranking (invariant 5, spec Clarifications).
- [ ] T035 [P] [US1] [U70] [U71] [U72] [U73] Add search cases to `crates/micold-client/tests/issue_source_state.rs`: a non-empty query on an incomplete listing → `SearchState::Pending { seq }`; on a complete listing or an empty query → `Idle` (invariant 6); `IssueSearchDue` acts only for the current `Pending { seq }` → `Searching`; a newer keystroke makes an older `IssueSearched` stale (FR-007a); `IssueSearched(Ok)` merges and re-ranks; `Err` → `SearchState::Failed` with the loaded matches kept; `IssueRetry` from `SearchState::Failed` → `Searching`; a searched issue can be picked (AS10); a searched issue that does not match the query by number, title or label is not in `issue_matches` (invariant 5).
- [ ] T036 [US1] [U62] [U74] Extend `crates/micold-client/tests/issues_are_requested_only_on_named_events.rs` so the search-due arm is the only other caller (FR-003), and add `main_tests.rs` shell tests named `issue…`: a keystroke schedules the 300 ms debounce, `IssueSearchDue` runs `search_open` with the `gh` path kept from the load.

- [ ] T094 [US1] [U95] Add to `crates/micold-core/tests/github_gh_cli.rs`: a stub `gh` that prints `search_pr_number.json` (JSON with `data` and a NOT_FOUND error) and exits 1 makes `GhCli::search_open` return the search hits, not an error; a stub exiting 1 with no JSON on stdout is classified.
- [ ] T076 [US1] [A10] Write acceptance test A10 in `crates/micold-client/src/main_tests.rs` (fake source with 1,000 loaded of 1,200 open and a scripted `search_open`); confirm it fails before T037.

### Implementation for User Story 1, slice C

- [ ] T037 [US1] [U66] [U67] [U68] [U69] [U70] [U95] In `crates/micold-core/src/github.rs`, add `IssueSource::search_open(&self, repo, text) -> Result<Vec<Issue>, IssueLoadError>`, the two query documents, `search_args`, `parse_search`, `merge_searched`, and `GhCli`'s partial-response rule (stdout parsed whenever it holds JSON with `data`, whatever the exit status); extend `FakeIssueSource`.
- [ ] T038 [US1] [U70] [U71] [U72] [U73] In `crates/micold-client/src/features/worktree_form.rs`, add `SearchState::{Pending, Searching, Failed}` and the `IssueSearchDue` / `IssueSearched` messages with invariants 4–6; displayed issues are loaded followed by `searched`.
- [ ] T039 [US1] [U74] In the shell (the file chosen in T027) and `crates/micold-client/src/main.rs`, add the 300 ms debounce task → `IssueSearchDue { seq }` and the `spawn_blocking(search_open)` task → `IssueSearched { seq, … }`, and the retry arm for a failed search.
- [ ] T040 [US1] [A10] In `crates/micold-client/src/ui/worktree_form.rs`, extend the cap caption with "— search also looks on GitHub.", render "Searching GitHub…" and "Search beyond the loaded issues failed — …" + **Retry** under the picker; register the "Issue source Loaded, Searching" covered state in `crates/micold-client/tests/support/covered_states.rs` and regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt`.
- [ ] T041 [US1] Add "Searching beyond the 1,000 loaded issues" to `docs/user-guide/worktrees-and-sessions.md`.
- [ ] T077 [US1] [A10] Outer loop green: A10 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T042 [US1] Run quickstart §B6 with the `visual-pass` skill and record it in `specs/034-github-issue-worktree/evidence/quickstart-b.md`.

**Checkpoint**: US1 complete (AS1–AS10).

---

## Phase 6: User Story 2 - The issue's labels choose the worktree type (Priority: P2)

**Goal**: A pick pre-selects the type of the first mapping entry whose label the issue carries, or
clears it; the default mapping is stored in `settings.json`.

**Independent Test**: quickstart §B4 (type `fix` for a `bug` issue); automated: `issue_types.rs`, `settings_issue_mapping.rs`, the US2 cases of `issue_source_state.rs`.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T043 [P] [US2] [U76] [U77] [U78] Write `crates/micold-core/tests/issue_types.rs` for `default_mapping` (`bug`→fix, `enhancement`→feat, `documentation`→docs, in that order — FR-021) and `type_for_labels`: AS1; AS2 mapping order wins whatever order the issue lists its labels; AS5 case ignored (`label.trim().to_lowercase()` both sides); no match → `None`; empty mapping → `None`; two labels mapping to one type allowed.
- [ ] T044 [P] [US2] [U80] [U81] [U82] Write `crates/micold-core/tests/settings_issue_mapping.rs` (contracts/issue-naming-and-typing.md §3): round trip; field absent → `default_mapping()`; `[]` stays `[]`; an entry with an unknown `type` token is dropped, the rest kept, and the file is not moved to `.bak`; a daemon-side `SettingsStore::update` that does not touch the field preserves it; `SETTINGS_VERSION` stays 4; no issue content is written (SC-006).
- [ ] T045 [P] [US2] [U83] [U84] Add US2 cases to `crates/micold-client/tests/issue_source_state.rs`: `IssuePicked { number, mapping }` sets `type_` from `type_for_labels` (AS1, AS2), replaces a selected type (AS6), clears it when no label matches (AS3, FR-014) so `can_submit()` reports the existing "type required"; a later hand-chosen type is kept (AS4, FR-015); a pick with mapping A then a pick with mapping B uses B and the first pick's type is not recomputed (FR-014a).
- [ ] T046 [P] [US2] [U85] [U86] In `crates/micold-client/tests/features_settings.rs`, test that `ValidSettings::into_settings()` carries `issue_label_types` and a theme-only save keeps the stored mapping; in `crates/micold-client/src/main_tests.rs`, a shell test named `issue…` that the pick reads the mapping from the settings store at that moment (default when there is no store).

- [ ] T078 [US2] [A11] [A12] [A13] [A14] [A15] [A16] [A17] Write acceptance tests A11–A17 in `crates/micold-client/src/main_tests.rs` (settings store seeded with the mapping under test); confirm each fails before T047 (A17 may pass already on slice B's `row_text` — record it as a regression pin).

### Implementation for User Story 2

- [ ] T047 [US2] [U76] [U77] [U78] Create `crates/micold-core/src/issue_types.rs` (`LabelTypeEntry { label, type_ }` serialised as `{"label", "type"}`, `default_mapping`, `type_for_labels`) and register it in `crates/micold-core/src/lib.rs`; give `ConventionalType` in `crates/micold-core/src/naming.rs` `Serialize`/`Deserialize` as its lowercase token.
- [ ] T048 [US2] [U80] [U81] [U82] In `crates/micold-core/src/settings.rs`, add `issue_label_types: Vec<LabelTypeEntry>` to `Settings` and `StoredSettings` with `#[serde(default = …)]` → `default_mapping()` and the unknown-token drop on read; `SETTINGS_VERSION` unchanged.
- [ ] T049 [US2] [U86] In `crates/micold-client/src/features/settings.rs`, give `ValidSettings` the `issue_label_types` field copied by `into_settings()` (loaded from the current settings until US3 adds the editor), and save it through `crates/micold-client/src/shell/persist.rs` with the other client-owned fields.
- [ ] T050 [US2] [U83] [U84] [U85] In `crates/micold-client/src/features/worktree_form.rs`, change `IssuePicked` to `{ number, mapping: Vec<LabelTypeEntry> }` and set or clear `type_` with `type_for_labels`, updating every existing `IssuePicked` construction in `crates/micold-client/tests/issue_source_state.rs` and `crates/micold-client/src/main_tests.rs` (slice B/C tests pass `mapping: vec![]` where the type is not under test and keep their assertions); in the shell's `IssueRowPicked` handling, fill `mapping` from `caps.settings().load().settings.issue_label_types`, or `default_mapping()` with no store (FR-014a).
- [ ] T051 [US2] Add "The issue's labels choose the type" with the default table to `docs/user-guide/worktrees-and-sessions.md`, and replace slice B's "type is chosen by hand" sentence.
- [ ] T079 [US2] [A11] Outer loop green: A11 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T080 [US2] [A12] Outer loop green: A12 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T081 [US2] [A13] Outer loop green: A13 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T082 [US2] [A14] Outer loop green: A14 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T083 [US2] [A15] Outer loop green: A15 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T084 [US2] [A16] Outer loop green: A16 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T085 [US2] [A17] Outer loop green: A17 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T052 [US2] Re-run quickstart §B4 (type `fix` from a `bug` label, labels visible on rows — AS7) with the `visual-pass` skill and record it in `specs/034-github-issue-worktree/evidence/quickstart-b.md`.

**Checkpoint**: US2 works with the default mapping; `mise run gate` passes.

---

## Phase 7: User Story 3 - Edit the label-to-type mapping in Settings (Priority: P3)

**Goal**: Settings → GitHub issues shows the ordered mapping and lets the user add, change, remove,
reorder and restore defaults, refusing blank or duplicate labels.

**Independent Test**: quickstart §B12; automated: `issue_types.rs` validation, `features_settings.rs`.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T053 [P] [US3] [U79] Add `validate_mapping` cases to `crates/micold-core/tests/issue_types.rs`: blank after trim → `MappingError { index, kind: Blank }`; `Bug` after `bug` → `Duplicate { of }` with the earlier index; the first offending entry in order is returned; same type twice is valid (FR-019).
- [ ] T054 [P] [US3] [U87] [U88] [U89] [U90] [U91] [U92] Extend `crates/micold-client/tests/features_settings.rs`: `SettingsSection::GithubIssues` is 5th in `ALL` with label "GitHub issues" (AS1); the draft loads the stored mapping, or the default when never edited (AS6); `IssueMappingAdded` appends `("", Feat)`, `IssueMappingLabelChanged`, `IssueMappingTypeChanged`, `IssueMappingRemoved`, `IssueMappingMoved(i, Up/Down)` (no-op at the ends) and `IssueMappingDefaultsRestored` edit the draft (AS2, AS3, AS6, FR-018); a blank or duplicate label refuses the save with `FieldError { field: FieldId::IssueMappingLabel(i), section: GithubIssues }` and writes nothing (AS5); a valid save writes the mapping in order (AS4, FR-020). Update `crates/micold-client/tests/settings_sections.rs` and `settings_rail.rs` for the fifth section.
- [ ] T055 [P] [US3] [U93] Extend `crates/micold-client/tests/icons_font.rs` coverage to `Icon::IssueMapping` (`label`), `Icon::MoveUp` (`arrow_upward`), `Icon::MoveDown` (`arrow_downward`) in the shipped font.

- [ ] T086 [US3] [A18] [A19] [A20] [A21] [A22] [A23] Write acceptance tests A18–A23 in `crates/micold-client/src/main_tests.rs` (Settings messages through `update_inner`, a temp settings store, a fresh store for A21); confirm each fails before T056.

### Implementation for User Story 3

- [ ] T056 [P] [US3] [U79] In `crates/micold-core/src/issue_types.rs`, add `validate_mapping`, `MappingError` and `MappingErrorKind`.
- [ ] T057 [P] [US3] [U93] In `crates/micold-client/src/icons.rs`, add `Icon::IssueMapping`, `Icon::MoveUp`, `Icon::MoveDown` on the existing Material Symbols glyphs.
- [ ] T058 [US3] [U87] [U88] [U89] [U90] [U91] [U92] In `crates/micold-client/src/features/settings.rs` and `crates/micold-client/src/features/window.rs`, add `SettingsSection::GithubIssues`, `SettingsDraft.github: GithubDraft { entries: Vec<(String, ConventionalType)> }`, the six `IssueMapping…` messages, `FieldId::IssueMappingLabel(usize)`, and `validate_mapping` inside `SettingsDraft::validate`; `ValidSettings` now takes the mapping from the draft.
- [ ] T059 [US3] [A18] [A22] [A23] Create `crates/micold-client/src/ui/settings/github.rs` and wire it in `crates/micold-client/src/ui/settings/mod.rs` per contracts/issue-picker-ui.md §4: explanatory text, one row per entry (`TextField` with its `FieldError`, `Select<ConventionalType>`, `IconButton` MoveUp/MoveDown disabled at the ends, Delete), **Add entry**, **Restore defaults**, the empty-mapping caption; Save/Cancel stay the view's.
- [ ] T060 [US3] Register the covered state "Settings, GitHub issues with three entries and one offending entry" in `crates/micold-client/tests/support/covered_states.rs` and regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt`.
- [ ] T061 [US3] Add the "GitHub issues" section to `docs/user-guide/settings.md` and link it from the labels section of `docs/user-guide/worktrees-and-sessions.md`.
- [ ] T087 [US3] [A18] Outer loop green: A18 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T088 [US3] [A19] Outer loop green: A19 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T089 [US3] [A20] Outer loop green: A20 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T090 [US3] [A21] Outer loop green: A21 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T091 [US3] [A22] Outer loop green: A22 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T092 [US3] [A23] Outer loop green: A23 passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue` in `crates/micold-client/src/main_tests.rs`
- [ ] T062 [US3] Run quickstart §B12 with the `visual-pass` skill and record it in `specs/034-github-issue-worktree/evidence/quickstart-b.md`.

**Checkpoint**: All three stories work; `mise run gate` passes.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T063 [P] Add to `docs/development/architecture.md` where the issue fetch runs (client, on the host) and why the daemon only answers `RemoteList` (research R4, R5).
- [ ] T064 Run quickstart §B10 (desktop launch), §B11 (sandbox placement) and §B13 (both colour schemes) and record them in `specs/034-github-issue-worktree/evidence/quickstart-b.md`; the macOS and Windows arms of §B10 are recorded on those hosts, or escalated as missing access.
- [ ] T065 Run `mise run gate` and `scripts/build-lock.sh cargo test --release -p micold-core --test typeahead_budget`; confirm every quickstart §A row passes.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Slice A (Phase 3)** has no prerequisite.
- **Slice B (Phase 4)** depends on slice A (the core types, `RemoteList`, `load_listing`, `name_from_title`).
- **Slice C (Phase 5)** depends on slice B (the form states and shell load).
- **US2 (Phase 6)** depends on slice C: T050 updates slice C's `IssuePicked` tests.
- **US3 (Phase 7)** depends on US2 (the stored mapping and `ValidSettings` field).
- **Polish (Phase 8)** depends on all stories.

### Within Each Phase

- Tests first, observed failing; then implementation in the listed order.
- Core types (`micold-core`) before reducer, reducer before shell, shell before UI.
- The user-guide task lands in the same phase as the behaviour it describes.

### Parallel Opportunities

- Slice A tests T001–T008 and T093 touch different files and run in parallel; T011 and T016 run beside T012–T015.
- Slice B tests T018/T019 (same file, write in sequence), T020, T021, T023 in parallel; T028 beside T024–T027.
- Slice C tests T033, T034, T035 in parallel.
- US2 tests T043–T046 in parallel.
- US3 tests T053–T055 and implementations T056, T057 in parallel.

## Parallel Example: User Story 1, slice A

```text
T001 process_run_bounded.rs   T002 github_remote.rs   T003 git_remotes.rs   T004 github_locate.rs
T005 github_parse.rs          T006 github_load.rs     T007 github_classify.rs  T008 naming_from_title.rs
```

## Implementation Strategy

MVP first: slices A and B deliver US1 AS1–AS9 (the whole request with the type chosen by hand).
Slice C completes US1 for repositories over the cap. US2 removes the manual type step; US3 lets teams
with their own labels edit the mapping. Each step merges on its own with `mise run gate` green.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — The issue source and remote discovery (US1 core)

- **Tasks**: T001–T009, T093, T010–T017
- **Deliverable**: On `main`, the daemon answers `RemoteList` (protocol 16) with a repository's own
  remote URLs, and `micold-core` can find `gh` as a desktop-launched app would, load up to 1,000
  open issues through it, classify every failure into FR-007's messages, and derive a worktree name
  from a title — all held by tests. No UI reaches it yet (milestones.md rule 6); M2 completes it.
- **Satisfies**: FR-002 (remote discovery and GitHub URL rule), FR-004 (paging and cap), FR-007
  (10 s bound, classification, messages), FR-010, FR-022, FR-025 (arguments), FR-026 (locating
  `gh`, local-config remotes)
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-daemon --test remote_list`
- **Depends on**: —

### M2 — Pick an open issue in the create-worktree form 🎯 MVP

- **Tasks**: T018–T031, T066–T075, T032
- **Deliverable**: On `main`, the create-worktree form offers **GitHub issue** (disabled with the
  reason when the repository has no github.com remote, and naming `owner/name` before it contacts
  GitHub); choosing it lists the open issues with loading, empty and failure + Retry states; typing
  narrows the list; picking fills ticket and name, and the worktree is created by the new-branch
  path. The type is still chosen by hand until M4 (a pick leaves it unchanged); the user guide says so.
  The user guide's "From a GitHub issue" section describes it.
- **Satisfies**: US1 acceptance scenarios 1–9; FR-001, FR-002, FR-003, FR-005, FR-006, FR-007,
  FR-007a, FR-008, FR-009, FR-010, FR-010a, FR-011, FR-012, FR-023, FR-024, FR-025; SC-001, SC-003, SC-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue`;
  `scripts/build-lock.sh cargo test -p micold-client --test issue_source_state`; `mise run gate`
  (named-events gate, ToggleChip, showcase and layout gates);
  `scripts/build-lock.sh cargo test --release -p micold-core --test typeahead_budget` (SC-003);
  quickstart §B1–B5, B7–B9 in `evidence/quickstart-b.md`
- **Depends on**: M1

### M3 — Search beyond the 1,000 loaded issues

- **Tasks**: T033–T036, T094, T076, T037–T042, T077
- **Deliverable**: On `main`, in a repository with more than 1,000 open issues, typing the number or
  title of an unloaded open issue finds it through GitHub's search (debounced, discarded when
  stale, failing without hiding the loaded matches), and it can be picked.
- **Satisfies**: US1 acceptance scenario 10; FR-005a, FR-007a (search), Edge "search beyond the
  load cap", "pull requests"
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core --test github_search --test github_parse --test github_gh_cli`;
  `scripts/build-lock.sh cargo test -p micold-client --test issue_source_state`;
  `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue`; `mise run gate`; quickstart §B6
- **Depends on**: M2

### M4 — The issue's labels choose the worktree type

- **Tasks**: T043–T052, T078–T085
- **Deliverable**: On `main`, picking an issue labelled `bug`, `enhancement` or `documentation`
  pre-selects `fix`, `feat` or `docs` from the default mapping stored in `settings.json`; an issue
  with no mapped label clears the type. The user guide explains the default table.
- **Satisfies**: US2 acceptance scenarios 1–7; FR-013, FR-014, FR-014a, FR-015, FR-016, FR-017,
  FR-020, FR-021; SC-002, SC-006
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core --test issue_types --test settings_issue_mapping`;
  `scripts/build-lock.sh cargo test -p micold-client --test issue_source_state --test features_settings`;
  `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue`; `mise run gate`; quickstart §B4
- **Depends on**: M3 (T050 updates slice C's `IssuePicked` tests)

### M5 — Edit the label-to-type mapping in Settings

- **Tasks**: T053–T062, T086–T092
- **Deliverable**: On `main`, Settings → GitHub issues lists the ordered mapping and lets the user
  add, change, remove, reorder and restore defaults, refusing a blank or duplicate label on the
  offending entry; a saved change types the next pick in every open project and survives a restart.
  The Settings guide documents it.
- **Satisfies**: US3 acceptance scenarios 1–6; FR-018, FR-019, FR-020; SC-005
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core --test issue_types`;
  `scripts/build-lock.sh cargo test -p micold-client --test features_settings --test settings_sections --test settings_rail --test icons_font`;
  `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue`; `mise run gate`; quickstart §B12
- **Depends on**: M4

### M6 — Cross-platform and architecture close-out

- **Tasks**: T063–T065
- **Deliverable**: `docs/development/architecture.md` records where the issue fetch runs and why,
  and `evidence/quickstart-b.md` records §B10 (desktop launch on Linux, macOS, Windows), §B11
  (sandbox placement) and §B13 (both colour schemes).
- **Satisfies**: FR-026 (recorded on every OS and placement); Principles VI, VII, VIII (recorded)
- **Verify**: read `evidence/quickstart-b.md`; `mise run gate`
- **Depends on**: M1–M5
