---
feature: 034-github-issue-worktree
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 23 # US1 1–10, US2 1–7, US3 1–6
planned_at: d6c2f33e
updated_at: d6c2f33e
suite_baseline: pending # measured by the first M1 cycle on its own base; see cycle-log.md
---

# Test List: Create a Worktree from a GitHub Issue

## Outer loop: acceptance behaviors

The profile's acceptance runner (`sandbox_real_*`) drives the container runtime and cannot see the
create-worktree form. So these are **client integration tests over the real update loop**, in
`crates/micold-client/src/main_tests.rs`: messages go through `update_inner` with test
`Capabilities` whose issue-source factory returns a `FakeIssueSource` and whose settings store is a
temp file, and results are read through the render-free readers the view calls (`preview()`,
`can_submit()`, the form fields, `source_caption()`, `issue_notice()`). That is the highest entry point
the repository can test without a display and without the network. The rendered half is quickstart
§B, run by `visual-pass`. Every test is named with `issue` so a milestone Verify filter selects it.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A1  | With `RemoteList` answering `origin` on github.com, the open form offers three sources and **GitHub issue** is enabled; with no GitHub remote it is disabled with "This repository has no GitHub remote." | US1-1, FR-001, FR-002 | example | TODO | `main_tests.rs::issue_the_form_offers_a_github_issue_source` |
| A2  | Choosing **GitHub issue** loads through the fake source and lists its open issues in the source's order (most recently updated first), each row showing number, title and labels | US1-2, FR-004 | example | TODO | `main_tests.rs::issue_choosing_the_source_lists_open_issues` |
| A3  | Typing a title fragment, a label name, then a number narrows the matches on each keystroke with no source call; Down/Enter picks without leaving the field | US1-3, FR-005 | example | TODO | `main_tests.rs::issue_typing_narrows_the_list_locally` |
| A4  | Picking #42 "Crash when opening empty project" sets ticket `42`, name the title, and `preview()` equals the new-branch preview for the same type/ticket/name | US1-4, FR-009, FR-010, FR-012 | example | TODO | `main_tests.rs::issue_a_pick_fills_ticket_and_name` |
| A5  | After a pick, editing name and ticket changes `preview()` and the create request uses the edited values | US1-5, FR-011 | example | TODO | `main_tests.rs::issue_picked_values_stay_editable` |
| A6  | Submitting after a pick sends the same create request as a new-branch submit with the same type/ticket/name, and a taken branch raises the existing conflict prompt | US1-6, FR-012 | example | TODO | `main_tests.rs::issue_submit_creates_like_a_new_branch` |
| A7  | While the load is in flight the form shows Loading, and switching to **New branch** works; the late result then changes nothing | US1-7, FR-006, FR-007a | example | TODO | `main_tests.rs::issue_the_form_stays_usable_while_loading` |
| A8  | With a typed ticket and name, or an earlier pick, a pick replaces both | US1-8, FR-010a | example | TODO | `main_tests.rs::issue_a_pick_replaces_ticket_and_name` |
| A9  | Before choosing, the caption under the switch reads "GitHub issue reads open issues of o/r from GitHub." and no source call was made; after choosing, the body notice names `o/r` | US1-9, FR-025, FR-003 | example | TODO | `main_tests.rs::issue_the_source_says_it_contacts_github_before_it_does` |
| A10 | With 1,000 loaded of 1,200 open, typing the number of an unloaded open issue shows loaded matches at once, then (after the debounce) the searched issue joins them once and can be picked | US1-10, FR-005a | example | TODO | `main_tests.rs::issue_search_finds_an_issue_beyond_the_cap` |
| A11 | With the default mapping, picking an issue labelled `bug` selects type `fix` | US2-1, FR-013, FR-021 | example | TODO | `main_tests.rs::issue_a_bug_label_selects_fix` |
| A12 | Mapping `bug` before `enhancement`: an issue labelled `enhancement, bug` selects `fix` | US2-2, FR-013, FR-017 | example | TODO | `main_tests.rs::issue_the_first_mapping_entry_wins` |
| A13 | With a type selected, picking an issue with no mapped label clears it and `can_submit()` reports "type required" | US2-3, FR-014 | example | TODO | `main_tests.rs::issue_an_unmapped_issue_clears_the_type` |
| A14 | After a label-selected type, choosing another type keeps the user's choice in the create request | US2-4, FR-015 | example | TODO | `main_tests.rs::issue_a_label_type_can_be_overridden` |
| A15 | A mapping entry `Bug` matches an issue label `bug` | US2-5, FR-013 | example | TODO | `main_tests.rs::issue_label_matching_ignores_case` |
| A16 | With a type selected, picking an issue carrying a mapped label replaces it | US2-6, FR-014 | example | TODO | `main_tests.rs::issue_a_mapped_label_replaces_the_selected_type` |
| A17 | An issue row's text includes its labels | US2-7, FR-004 | example | TODO | `main_tests.rs::issue_rows_show_labels` |
| A18 | Opening Settings → GitHub issues shows the stored mapping as ordered label → type entries | US3-1, FR-018 | example | TODO | `main_tests.rs::issue_settings_shows_the_mapping` |
| A19 | Adding `defect → fix` and saving, then picking a `defect` issue in a project selects `fix` without restart | US3-2, FR-016, SC-005 | example | TODO | `main_tests.rs::issue_an_added_entry_types_the_next_pick` |
| A20 | Changing, removing and reordering entries then saving types the next pick by the new mapping | US3-3, FR-017, FR-018 | example | TODO | `main_tests.rs::issue_edited_mapping_types_the_next_pick` |
| A21 | A saved mapping is read back from `settings.json` by a fresh store (restart) | US3-4, FR-020 | example | TODO | `main_tests.rs::issue_the_mapping_survives_a_restart` |
| A22 | A blank label, or `Bug` beside `bug`, refuses the save with an error on that entry and the file is unchanged | US3-5, FR-019 | example | TODO | `main_tests.rs::issue_an_invalid_mapping_is_not_saved` |
| A23 | Never edited → the default three entries are shown; after edits, Restore defaults returns them | US3-6, FR-021, FR-018 | example | TODO | `main_tests.rs::issue_restore_defaults_returns_the_default_mapping` |

## Inner loop: unit behaviors

### `crates/micold-core/src/process.rs`: `run_bounded`

Tests: `crates/micold-core/tests/process_run_bounded.rs` (new).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U1  | A child sleeping past the bound returns `TimedOut` within bound + 1 s and is killed | FR-007, SC-004, R6 | example | DONE | `process_run_bounded.rs::a_child_past_the_bound_is_killed_and_reported` |
| U2  | A child finishing just inside the bound returns `Exited` (other side of the boundary) | FR-007 | example | DONE | `process_run_bounded.rs::a_child_inside_the_bound_exits_normally` |
| U3  | A child writing 1 MiB stdout and 64 KiB stderr returns every byte of both, not `TimedOut` | FR-007, R6 | example | DONE | `process_run_bounded.rs::large_output_is_drained_while_waiting` |
| U4  | A non-zero exit returns its status with stdout and stderr | FR-007 | example | DONE | `process_run_bounded.rs::a_failing_child_reports_status_and_output` |
| U5  | The existing env-include suites stay green after the move | R6 | characterization | DONE | `crates/micold-core/tests/env_include*.rs` (existing) |

### `crates/micold-core/src/git.rs`: `remote_list`, `parse_remote_list`

Tests: `crates/micold-core/tests/git_remotes.rs` (new).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U6  | `remote.<name>.url <url>` lines parse to `GitRemote`s in config order | FR-002, R5 | example | DONE | `git_remotes.rs::lines_parse_in_config_order` |
| U7  | A dotted remote name (`remote.a.b.url`) parses to name `a.b` | R5 | example | DONE | `git_remotes.rs::a_dotted_remote_name_is_kept_whole` |
| U8  | A second `url` for one remote is ignored (first wins); empty output → empty list | R5 | example | DONE | `git_remotes.rs::first_url_wins_and_empty_is_empty` |
| U9  | `GitCli::remote_list` on a temp repo with two remotes lists both; with none returns `Ok("")` | FR-002 | example | DONE | `git_remotes.rs::git_cli_lists_remotes_and_none_is_not_an_error` |
| U10 | A global `insteadOf` rewrite is not applied to the listed URL | FR-026, R5 | example | DONE | `git_remotes.rs::global_insteadof_is_not_applied` |
| U97 | `FakeGit::with_remote` lists remotes in the order added, as git lists config order (added in M1 cycle 2: T011 names the fake) | R5 | example | DONE | `git_remotes.rs::fake_git_lists_remotes_in_insertion_order` |

### `crates/micold-core/src/github.rs`: `GithubRepo`, `choose_remote`

Tests: `crates/micold-core/tests/github_remote.rs` (new).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U11 | Every accepted github.com URL form of R5 (https/http/scp/ssh/git, `.git`, trailing `/`, `ssh.github.com:443`, `GITHUB.COM`) yields `owner/name` | FR-002, R5 | example | DONE | `github_remote.rs::accepted_url_forms` |
| U12 | Userinfo (`user@`, `x-access-token:T@`) is discarded; the token never appears in the value or `Display` | FR-022, SC-006 | example | DONE | `github_remote.rs::userinfo_is_discarded` |
| U13 | `www.github.com`, `github.example.com`, GitLab, Bitbucket, local paths, `gh:o/r` are rejected | FR-002 | example | DONE | `github_remote.rs::non_github_urls_are_rejected` |
| U14 | `origin` on GitHub wins over an earlier GitHub remote | Edge "several remotes" | example | DONE | `github_remote.rs::origin_wins_when_on_github` |
| U15 | origin on GitLab + upstream on GitHub picks upstream; two GitHub remotes without origin picks the first | Edge "several remotes" | example | DONE | `github_remote.rs::first_github_remote_otherwise` |
| U16 | No remote, or none on github.com → `NoGithubRemote` | FR-002 | example | DONE | `github_remote.rs::no_github_remote` |

### `crates/micold-core/src/github.rs`: `HostOs`, `locate_gh`

Tests: `crates/micold-core/tests/github_locate.rs` (new), all against a fake `exists`.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U17 | Each `HostOs` has its separator (`:`/`;`) and executable name (`gh`/`gh.exe`) regardless of host | FR-026, R3 | example | DONE | `github_locate.rs::separator_and_exe_come_from_host_os` |
| U18 | Env-include `PATH` before process `PATH` before well-known dirs; duplicates keep first position; empty components dropped | FR-026, R3 | example | DONE | `github_locate.rs::candidate_order` |
| U19 | `MacOs` with the Dock `PATH` finds `gh` only in `/opt/homebrew/bin` | FR-026, Edge "desktop launch" | example | DONE | `github_locate.rs::macos_dock_launch_finds_homebrew_gh` |
| U20 | `Windows` finds `gh.exe` only under `%LOCALAPPDATA%\Microsoft\WinGet\Links` | FR-026 | example | DONE | `github_locate.rs::windows_finds_winget_gh` |
| U21 | `Linux` well-known table (e.g. `~/.local/bin`, `/home/linuxbrew/.linuxbrew/bin`, `/snap/bin`) is searched | FR-026 | example | DONE | `github_locate.rs::linux_well_known_dirs` |
| U22 | A `Path` (not `PATH`) key in the env-include snapshot is honoured | FR-026, R3 | example | DONE | `github_locate.rs::path_key_is_matched_ignoring_case` |
| U23 | Nothing found → `None` | Edge "tooling not installed" | example | DONE | `github_locate.rs::none_when_absent` |

### `crates/micold-core/src/github.rs`: issues, paging, parsing, arguments

Tests: `crates/micold-core/tests/github_parse.rs`, `github_load.rs` (new), fixtures under `tests/fixtures/gh/`.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U24 | `parse_list_page` maps number, title, updatedAt, ≤ 20 labels, `totalCount`, `pageInfo` | FR-004 | example | DONE | `github_parse.rs::a_list_page_parses` |
| U25 | `row_text` is `#n title` with `  ·  l1, l2` only when labelled | FR-004, US2-7 | example | DONE | `github_parse.rs::row_text_shows_labels_only_when_present` |
| U26 | `errors[]` NOT_FOUND → `NoAccess`; RATE_LIMITED → `RateLimited`; malformed → `Other` | FR-007 | example | DONE | `github_parse.rs::graphql_errors_are_classified` |
| U27 | `list_args` uses `-f` for every string, `-f cursor` only with a cursor; repo `1`/`true` stays a string; nothing but owner/name/cursor is sent | FR-025, R2 | example | DONE | `github_parse.rs::list_args_send_only_the_repository` |
| U28 | Pages concatenate in order until no next cursor; `complete` when held ≥ total | FR-004 | example | DONE | `github_load.rs::pages_concatenate_until_the_last` |
| U29 | At exactly 1,000 held paging stops, the last page is truncated, `complete` false with total 1,001; with total 1,000 `complete` true (both sides) | FR-004 | example | DONE | `github_load.rs::the_cap_is_1000` |
| U30 | The first error aborts the load and is returned; zero open → empty complete listing | FR-007, FR-008 | example | DONE | `github_load.rs::first_error_aborts_and_empty_is_complete` |

### `crates/micold-core/src/github.rs`: `classify`, `IssueLoadError::message`

Tests: `crates/micold-core/tests/github_classify.rs` (new), `tests/fixtures/gh/*.stderr`.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U31 | Exit 4 / "gh auth login" / HTTP 401 → `NotSignedIn` | FR-007, FR-022 | example | TODO | `github_classify.rs::not_signed_in` |
| U32 | 404, 403, SAML → `NoAccess` | FR-007 | example | TODO | `github_classify.rs::no_access` |
| U33 | DNS/connection failures → `Offline`; rate-limit text → `RateLimited`; `TimedOut` outcome → `TimedOut` | FR-007, SC-004 | example | TODO | `github_classify.rs::offline_rate_limited_timed_out` |
| U34 | Unknown stderr → `Other(first non-empty line)` | FR-007 | example | TODO | `github_classify.rs::unknown_text_is_other` |
| U35 | `message(repo)` returns the §5 text per variant, `ToolMissing` included, naming `owner/name` for `NoAccess` | FR-007, Edge "tooling not installed" | example | TODO | `github_classify.rs::messages_name_cause_and_remedy` |

### `crates/micold-core/src/naming.rs`: `name_from_title`

Tests: `crates/micold-core/tests/naming_from_title.rs` (new).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U36 | A title whose slug is ≤ 50 is kept (whitespace normalised); a title whose slug is exactly 50 is kept whole and one at 51 is cut (both sides) | FR-010 | example | TODO | `naming_from_title.rs::fits_and_the_50_boundary` |
| U37 | A long title is cut to the longest whole-word prefix with slug ≤ 50 | FR-010 | example | TODO | `naming_from_title.rs::cut_at_word_boundary` |
| U38 | A first word longer than 50 slug characters yields the first 50 characters of the slug | FR-010 | example | TODO | `naming_from_title.rs::a_long_first_word_is_cut_at_50` |
| U39 | A title that slugs to nothing yields `""` | FR-010, Edge "slugs to nothing" | example | TODO | `naming_from_title.rs::empty_slug_yields_empty_name` |
| U40 | `slugify(name_from_title(t)).len() <= 50` over a corpus | FR-010 | property | TODO | `naming_from_title.rs::slug_never_exceeds_50` (sampled at the boundaries; no property library) |

### Protocol and daemon: `RemoteList`

Tests: `crates/micold-daemon/tests/remote_list.rs` (new), the core protocol round-trip test, `crates/micold-core/tests/schema_hash.rs`.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U41 | A repo with `origin` (GitHub) and `upstream` answers `OperationResult::RemoteList` with both in order | FR-002 | example | TODO | `remote_list.rs::a_repository_answers_its_remotes` |
| U42 | A non-repository project is rejected like `BranchList` | FR-002 | example | TODO | `remote_list.rs::a_non_repository_is_rejected` |
| U43 | `ClientMsg::RemoteList` and `OperationResult::RemoteList` round-trip; `PROTOCOL_VERSION` is 16 | R5 | example | TODO | protocol round-trip test + `schema_hash.rs` pin |

### `crates/micold-core/src/github.rs`: `GhCli`

Tests: `crates/micold-core/tests/github_gh_cli.rs` (new), against a stub `gh` the test writes.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U94 | `GhCli::list_open` passes exactly `list_args`, sets the five `gh` environment variables, runs in the user's home, parses the page; a stub past a short bound → `TimedOut`; exit 4 + not-logged-in text → `NotSignedIn` | FR-007, FR-022, FR-025, R6 | example | TODO | `github_gh_cli.rs::gh_cli_runs_gh_as_specified` |

### `crates/micold-client/src/features/worktree_form.rs`: availability, load, staleness

Tests: `crates/micold-client/tests/issue_source_state.rs` (new).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U96 | `Opened` starts with `github = Checking` and `issues = NotRequested`; `source_caption()` gives the Checking text, the Unavailable reason, or — while `Available` and the source is not `Issue` — the opt-in notice naming `owner/name`; `issue_notice()` names `owner/name` while the source is `Issue` | FR-002, FR-025 | example | TODO | `issue_source_state.rs::captions_follow_availability` |
| U44 | `RemotesListed(Ok)` → `Available(repo)`; no GitHub remote → `Unavailable("This repository has no GitHub remote.")`; `Err(d)` → `Unavailable("Couldn't read this repository's remotes: d")` | FR-002 | example | TODO | `issue_source_state.rs::remotes_decide_availability` |
| U45 | `SourceChanged(Issue)` is refused while `Checking` or `Unavailable`, accepted while `Available` → `Loading { seq }` | FR-003, inv. 1 | example | TODO | `issue_source_state.rs::the_source_is_chosen_only_when_available` |
| U46 | `IssuesLoaded` with the awaited seq applies; with any other seq is dropped; with no form open is dropped | FR-007a, inv. 2 | example | TODO | `issue_source_state.rs::only_the_awaited_result_applies` |
| U47 | `IssueRetry` from `Failed` → `Loading` with a new seq; from `NotRequested` or `Loaded` does nothing | FR-007, inv. 1 | example | TODO | `issue_source_state.rs::retry_only_from_failed` |
| U48 | Switching away keeps type/ticket/name, returns to `NotRequested`, and a late result is dropped | Edges "switch mid-load", "switch back", inv. 7 | example | TODO | `issue_source_state.rs::switching_away_keeps_fields_and_drops_the_load` |
| U49 | A zero-issue listing is the empty state | FR-008 | example | TODO | `issue_source_state.rs::no_open_issues` |
| U50 | Closing and reopening the form starts at `NotRequested` with no issues held | FR-023, Edge "form closed" | example | TODO | `issue_source_state.rs::closing_the_form_forgets_issues` |
| U51 | `issue_request_seq` is never reset: a result for a closed form's seq does not apply to a new form | FR-007a, R9 | example | TODO | `issue_source_state.rs::a_closed_forms_result_never_matches_a_new_form` |

### `crates/micold-client/src/features/worktree_form.rs`: ranking and the pick

Tests: `crates/micold-client/tests/issue_source_state.rs`.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U52 | `IssueQueryChanged("42")` ranks `#42` first; a label name finds its issue; a title fragment narrows | FR-005 | example | TODO | `issue_source_state.rs::the_query_ranks_row_text` |
| U53 | `issue_highlight < issue_matches.len()` through move/focus/dismiss and re-rank | inv. 3 | example | TODO | `issue_source_state.rs::highlight_stays_in_range` |
| U54 | `issue_number_at(i)` returns the number at `issue_matches[i]`; `None` at `len()` (both sides) | R12 | example | TODO | `issue_source_state.rs::issue_number_at_bounds` |
| U55 | `IssuePicked` sets ticket (no `#`), name via `name_from_title`, clears error, closes the list, marks `picked_issue` | FR-009, FR-010 | example | TODO | `issue_source_state.rs::a_pick_fills_ticket_and_name` |
| U56 | `IssuePicked` for a number not held, or on a form not `Loaded`, is a no-op | contract §4 | example | TODO | `issue_source_state.rs::a_stale_pick_is_ignored` |
| U57 | `preview()`/`can_submit()` for `Issue` equal those for `New` with the same fields | FR-012 | example | TODO | `issue_source_state.rs::issue_source_previews_as_new` |

### `crates/micold-client/src/shell/` and `main.rs`: RemoteList, load, pick

Tests: `crates/micold-client/src/main_tests.rs`, each named with `issue`; `crates/micold-client/tests/issues_are_requested_only_on_named_events.rs` (new).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U58 | Opening the form sends one `RemoteList` for the active project; OK/Err route to `RemotesListed`; a reply for an inactive project is dropped | FR-002 | example | TODO | `main_tests.rs::issue_opening_the_form_asks_for_remotes` |
| U59 | Not connected → `RemotesListed(Err("not connected to the session service"))`, no toast; a dropped connection resolves a pending `RemoteList` the same way | FR-002, FR-024 | example | TODO | `main_tests.rs::issue_remotes_without_a_connection` |
| U60 | A cache miss resolves the env-include snapshot inside the load and it lands in `App::env_include_cache`; a hit is reused | FR-026, R3 | example | TODO | `main_tests.rs::issue_the_load_uses_the_env_include_path` |
| U61 | `gh` not located → `Failed(ToolMissing)` and no source is constructed | Edge "tooling not installed" | example | TODO | `main_tests.rs::issue_missing_gh_is_reported_without_running` |
| U62 | The issue-source capability is called only from the load, retry and search-due arms | FR-003, FR-024 | example | TODO | `issues_are_requested_only_on_named_events.rs` |
| U63 | `GhCli` is named only in `Capabilities::real()` | Principle I | example | TODO | `no_concrete_implementations.rs` (existing gate, extended by construction) |

### `crates/micold-client/src/ui/material/toggle_chip.rs`

Tests: unit tests in `toggle_chip.rs`, `material_builder_api.rs`, `showcase_completeness.rs`.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U64 | A disabled chip emits no press; an enabled one does (both sides) | FR-002, Principle VIII | example | TODO | `toggle_chip.rs::a_disabled_chip_emits_no_press` |
| U65 | `.disabled(bool)` is chainable and the gallery poses it | Principle VIII | example | TODO | `material_builder_api.rs`, `showcase_completeness.rs` |

### Search beyond the cap (`github.rs`, `worktree_form.rs`, shell)

Tests: `crates/micold-core/tests/github_parse.rs`, `github_search.rs` (new), `crates/micold-client/tests/issue_source_state.rs`, `main_tests.rs`.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U66 | `parse_search` unions `search.nodes` and `repository.issue`, open only, deduped by number | FR-005a | example | TODO | `github_parse.rs::search_unions_and_dedupes` |
| U67 | A sole NOT_FOUND at `["repository","issue"]` (PR number, missing number) keeps the hits without error; a closed numbered issue is dropped | FR-005a, Edge "pull requests" | example | TODO | `github_parse.rs::a_missing_number_is_not_an_error` |
| U68 | `search_args`: plain text uses SEARCH_QUERY with only `-f q=repo:o/n is:issue is:open <text>`; `N`/`#N` uses SEARCH_WITH_NUMBER_QUERY with `-F n=N`; a number over `Int` range falls back to SEARCH_QUERY | FR-025, R2 | example | TODO | `github_parse.rs::search_args` |
| U69 | `merge_searched` drops numbers already loaded | FR-005a, inv. 4 | example | TODO | `github_search.rs::merge_drops_loaded_numbers` |
| U70 | A searched issue that does not match by number, title or label is not displayed: `typeahead::rank` over `row_text` drops it (core), and the reducer leaves it out of `issue_matches` | FR-005a, inv. 5 | example | TODO | `github_search.rs::a_body_only_match_is_hidden` + `issue_source_state.rs::an_unmatched_searched_issue_is_hidden` |
| U71 | Non-empty query on an incomplete listing → `Pending`; on a complete listing or an empty query → `Idle` | FR-005a, inv. 6 | example | TODO | `issue_source_state.rs::search_only_when_incomplete` |
| U72 | `IssueSearchDue` acts only for the current `Pending` seq; an older `IssueSearched` is dropped | FR-007a | example | TODO | `issue_source_state.rs::a_newer_keystroke_discards_an_older_search` |
| U73 | `IssueSearched(Err)` → `SearchState::Failed` with loaded matches kept; `IssueRetry` → `Searching` | FR-005a, FR-007 | example | TODO | `issue_source_state.rs::a_failed_search_keeps_loaded_matches` |
| U74 | A keystroke schedules one 300 ms debounce; `IssueSearchDue` runs `search_open` with the load's `gh` path | R9 | example | TODO | `main_tests.rs::issue_search_is_debounced` |
| U95 | `GhCli` returns stdout that parses as JSON with `data` whatever the exit status (partial response); non-zero exit without JSON goes to `classify` | FR-005a, R2 | example | TODO | `github_gh_cli.rs::a_partial_response_is_parsed` |
| U75 | Ranking 1,000 issue rows for a 3-character query takes < 50 ms in release | SC-003 | example | TODO | `typeahead_budget.rs` (new case) |

### `crates/micold-core/src/issue_types.rs`, `settings.rs`

Tests: `crates/micold-core/tests/issue_types.rs`, `settings_issue_mapping.rs` (new).

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U76 | `default_mapping()` is `bug→fix, enhancement→feat, documentation→docs` in that order | FR-021 | example | TODO | `issue_types.rs::default_mapping` |
| U77 | `type_for_labels` returns the first mapping entry matching any label, whatever the label order | FR-013 | example | TODO | `issue_types.rs::mapping_order_wins` |
| U78 | Matching trims and ignores case; no match or empty mapping → `None`; two labels to one type allowed | FR-013, FR-014 | example | TODO | `issue_types.rs::case_and_no_match` |
| U79 | `validate_mapping`: blank after trim → `Blank` at its index; `Bug` after `bug` → `Duplicate { of }`; first offender returned; same type twice valid | FR-019 | example | TODO | `issue_types.rs::validation` |
| U80 | Settings round-trip the mapping; absent → default; `[]` stays `[]` | FR-020, FR-021 | example | TODO | `settings_issue_mapping.rs::round_trip_and_default` |
| U81 | An unknown type token drops that entry only; the file is not moved to `.bak` | R10 | example | TODO | `settings_issue_mapping.rs::unknown_type_is_dropped` |
| U82 | A daemon-side `update` not touching the field preserves it; `SETTINGS_VERSION` stays 4; no issue content is written | FR-016, SC-006 | example | TODO | `settings_issue_mapping.rs::other_writers_preserve_the_mapping` |

### `crates/micold-client/src/features/worktree_form.rs` + shell: type from labels

Tests: `crates/micold-client/tests/issue_source_state.rs`, `features_settings.rs`, `main_tests.rs`.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U83 | `IssuePicked { mapping }` sets `type_` from a match, replaces a selected type, clears it on no match | FR-013, FR-014 | example | TODO | `issue_source_state.rs::the_pick_sets_or_clears_the_type` |
| U84 | A pick with mapping A then one with mapping B uses B; the first pick's type is not recomputed | FR-014a | example | TODO | `issue_source_state.rs::the_mapping_is_read_at_the_pick` |
| U85 | The shell fills `mapping` from the settings store at the pick, `default_mapping()` with no store | FR-014a | example | TODO | `main_tests.rs::issue_the_pick_reads_the_stored_mapping` |
| U86 | `ValidSettings::into_settings()` carries the mapping; a theme-only save keeps it | FR-016, FR-020 | example | TODO | `features_settings.rs::the_mapping_survives_other_saves` |

### `crates/micold-client/src/features/settings.rs`: GitHub issues section

Tests: `crates/micold-client/tests/features_settings.rs`, `settings_sections.rs`, `settings_rail.rs`, `icons_font.rs`.

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U87 | `GithubIssues` is 5th in `ALL`, labelled "GitHub issues", icon `IssueMapping` | FR-018 | example | TODO | `settings_sections.rs`, `settings_rail.rs` |
| U88 | The draft loads the stored mapping, or the default when absent | FR-021 | example | TODO | `features_settings.rs::the_draft_loads_the_mapping` |
| U89 | Add appends `("", Feat)`; label/type change edit in place; remove deletes | FR-018 | example | TODO | `features_settings.rs::entries_are_edited` |
| U90 | Move up at index 0 and move down at the last index are no-ops; elsewhere they swap (both sides) | FR-017, FR-018 | example | TODO | `features_settings.rs::entries_are_reordered` |
| U91 | Restore defaults replaces the draft with `default_mapping()` | FR-018 | example | TODO | `features_settings.rs::restore_defaults` |
| U92 | An invalid mapping maps to `FieldError { IssueMappingLabel(i), GithubIssues }` and nothing is written | FR-019 | example | TODO | `features_settings.rs::an_invalid_mapping_refuses_the_save` |
| U93 | `IssueMapping`, `MoveUp`, `MoveDown` glyphs are in the shipped font | Principle VIII | example | TODO | `icons_font.rs` |

## Invariants and edge cases still to place

None. "Session sandbox" and "desktop launch" (FR-026) are held structurally by U10, U17–U22 and
U60 and observed end to end by quickstart §B10–B11. "Issue closed while the list is open" needs no
test: the list is a snapshot and a pick never re-reads GitHub. "Many open issues" is U75. "Mapping
maps two labels to one type" is U78.

## Out of scope

- GitHub Enterprise and non-github.com hosts (spec Clarifications).
- Anonymous access without a sign-in (FR-022).
- A per-project mapping; linking the worktree back to its issue (spec Assumptions).
- Remotes named through a global `insteadOf` alias (research R5; U10 pins the non-support).

## Verification commands

From `.specify/memory/tdd-profile.md`, run from the workspace root:

- single: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- file: `scripts/build-lock.sh cargo test --test {file}`
- suite: `scripts/build-lock.sh cargo test --workspace`
- core-only inner loop: `mise run test-core`
- binary-module tests (`main_tests.rs`): `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide issue`
- release budget: `scripts/build-lock.sh cargo test --release -p micold-core --test typeahead_budget`
