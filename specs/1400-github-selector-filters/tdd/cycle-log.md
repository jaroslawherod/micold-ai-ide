# TDD cycle log: 1400

The feature has no `tdd/test-list.md` (as 1186): tasks.md's test tasks are the list, and each row
names the task pair it covers.

M1's tests and implementation landed in one commit (`a3e43dbac`, rebased as `04e671637`), so its
red runs were reconstructed after the fact (M1 review B round 1 F1), two ways, on that commit:

- **structural**: `crates/micold-core/src` put back to its pre-M1 state (`ed945a97b`, without
  `github_filter.rs` and `query_text.rs`) under M1's test files, then
  `cargo test -p micold-core --test github_filter --test github_gh_cli --no-fail-fast`;
- **mutant**: M1's source with one mutant per task (named in its row), all seven applied together,
  the same command: 11 of 47 `github_filter` tests and 1 of 17 `github_gh_cli` tests FAILED, each
  mutant failing its own task's tests.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001 | `github_filter` and `query_text` modules, `lib.rs` declarations, empty test file | structural | `cargo test -p micold-core --test github_filter` compiles |
| T002 | `clean`/`cut_message` moved to `query_text` | refactor, behaviour-preserving | `jira_query` 36 passed unchanged |
| T003 (red), T004 | `LabelName::parse` (§5), `IssueState` default/serde, `BasicFilters` setters, count, line, lenient read, redacted `Debug` | structural: `error[E0432]: unresolved import micold_core::github_filter`; mutant (`c == '"'` dropped from `LabelName::parse`): `a_label_name_is_any_non_empty_name_without_a_quote_or_control_character` FAILED `left: Some(<redacted>) right: None`, `basic_filters_read_leniently` FAILED `left: ["bug", "say \"hi\"", "both"] right: ["bug", "both"]`, `label_choices_are_sorted_without_case_and_held_once`, `invalid_remembered_values_are_dropped` FAILED | `github_filter` 47 passed |
| T005 (red), T006 | `UserQuery::parse`: cleaning, 1,000-char limit, refusals and their priority (§1, §2), sort split (§3), `QueryRefusal::message` | structural (as T003); mutant (`scope` never set in `scan`): `a_scope_qualifier_is_refused` FAILED | same |
| T007 (red), T008 | `GithubFilter::is_default`, `ScopedSearch::listing`/`beyond` (§4), redacted `Debug` | structural (as T003); mutant (`beyond` keeps `"`): `the_search_beyond_appends_the_typed_words_quoted` FAILED `left: … "say" ""hi""" right: … "say" "hi"`, `no_search_beyond_is_sent_without_a_word` FAILED `left: Some(… """") right: None` | same |
| T009 (red), T010 | `parse_search_page` (§6): order, `is_closed()`, other repositories and pull requests dropped, `Rejected` in order, `RATE_LIMITED`, messages cut/cleaned/counted, other shapes `Load(Other)` | structural: `no associated function or constant named is_closed found for struct Issue`, `unresolved import micold_core::github_filter`; mutant (any non-empty repository kept): `a_search_page_drops_other_repositories_and_pull_requests` FAILED, `issues_of_other_repositories_are_dropped_before_the_cap_counts` FAILED `left: [1, 2] right: [2]` | same |
| T011 | `IssueSource::{search_page, search_filtered, labels_page}` default bodies, `NamePage`, fake builders | seam for T012; structural: `no method named with_search_pages found for struct FakeIssueSource`, `unresolved import micold_core::github::NamePage` | covered by T012 |
| T012 (red), T013 | `load_filtered` paging, 1,000 cap, `complete`, `total_open`, first failure aborts, `Rejected` passes; `load_label_choices` sorted, held once, 10-page cap | structural: `no method named with_search_pages` / `with_labels found for struct FakeIssueSource`; mutant (`complete: true`): `a_filtered_load_holds_at_most_the_cap` FAILED | same |
| T014 (red), T015 | `RememberedSelector` round trip, lenient read, refused query → empty and not ran, `with_change`, `github_selector_path` | structural: `unresolved import micold_core::store::github_selector_path`; mutant (a refused query kept as read): `invalid_remembered_values_are_dropped` FAILED, `each_change_sets_one_field` FAILED | same |
| T016 (red), T017 | `GhCli::{search_page, search_filtered, labels_page}` argument vectors (`-f` only, cursor only when given), a parser `Rejected` kept on exit 1, stderr-only failures classified as 034 | structural: `no method named search_page` / `search_filtered` / `labels_page found for struct GhCli`, `unresolved import micold_core::github::labels_args`; mutant (cursor never sent): `a_filtered_page_sends_the_scoped_text_and_the_cursor` FAILED `left: ["-f", "q=…"] right: ["-f", "q=…", "-f", "cursor=CUR"]` | `github_gh_cli` 17 passed |
| T018 | `ISSUE_ADVANCED` and refusal shape probe | not probed (no signed-in `gh`): deviation in `autopilot.md` | — |
| review A F1 | `LabelName::parse` refuses `\` (`foo\`, `a\b`), so `label:"foo\"` can never leave its quote open | test written first, run before the fix: `a_label_name_is_any_non_empty_name_without_a_quote_backslash_or_control_character` FAILED `left: Some(<redacted>) right: None` | `github_filter` 48 passed |
| review A F2 | a scope qualifier after any character but a letter, digit, `_` or `-` is refused (`(a)repo:other/x`, `"x"repo:other/x`, `label:a,repo:other/x`); `myrepo:x`, `my_repo:x`, `my-repo:x` stay `Ok` | test first: `a_scope_qualifier_is_refused` FAILED (`(a)repo:other/x` parsed) | same |
| review A F3 | `FORBIDDEN` and `INSUFFICIENT_SCOPES` beside a refusal are `Load(graphql_error_of(..))`, not `Rejected`; through `GhCli` a SAML refusal is `NoAccess` | test first: `a_refused_access_or_missing_scope_is_a_load_failure_not_a_rejection` FAILED `left: Rejected(2 message(s)) right: Load(Other("Resource protected by SAML"))`; `a_saml_refused_search_is_no_access_not_a_rejection` FAILED `left: Rejected(1 message(s)) right: Load(NoAccess)` | `github_filter` 48, `github_gh_cli` 18 passed |

## M2 (T019–T030)

M2's tests were written before the code they drive, in this order: `tests/github_filter_state.rs`
(T020–T022) first, then the reducer (T023), the shell (T025, T024's scan beside it), the view
(T026, T028) and its gates (T027). The cloud unit could not afford a build per step (one client
build is minutes behind the shared build lock), so the first run came after the code; red is shown
the way M1's was reconstructed, by mutants, all applied together on the finished code and run with
`cargo test -p micold-client --test github_filter_state --test
issues_are_requested_only_on_named_events --no-fail-fast`: 10 of 22 `github_filter_state` tests
and 2 of 4 named-events tests FAILED, each mutant failing its own task's tests. Green: the same
command, 22 and 4 passed; `jira_filters_picker` 52 passed unchanged (T019, T026).

| Task | Behaviour | Red evidence (mutant → failing test) | Green |
|---|---|---|---|
| T019 | `ChoiceLoad::{start, answer, awaited}` `pub(super)` | refactor, no behaviour change | `jira_filters_picker` 52 passed |
| T020 (red), T023, T025 | FR-009 default filter → 034's listing, no scoped search | `GithubSelector`'s filter kept when default (`.filter(\|_\| true)`): `the_default_filter_loads_034s_listing` FAILED (`left: Some((1, GithubRepo { owner: "o", name: "r" }, ScopedSearch("<redacted>"))) right: None`), `no_match_with_a_filter_says_the_filter_is_active`, `an_empty_filtered_list_says_no_issue_matches_the_filters` FAILED | `github_filter_state` 22 passed |
| T020 (red), T023 | FR-008 removing a label / Clear filters reloads | a removed chip reloads nothing (`reload = false`): `removing_a_value_and_clear_filters` FAILED (`called Option::unwrap() on a None value`: no filtered listing awaited) | same |
| T021 (red), T023 | FR-005 labels asked once, on the first focus | every opening asks again (`labels = Loading { seq }; true`): `the_first_focus_of_a_label_field_loads_the_labels_once` FAILED (`left: Some(3) right: None`) | same |
| T021 (red), T023 | US1 AS1 choices less the labels chosen in either list | chosen labels still offered: `the_choices_are_alphabetical_less_the_chosen`, `a_label_chosen_in_one_list_is_offered_in_neither` FAILED | same |
| T022 (red), T023 | FR-018 `Rejected(m)` → `IssueList::Rejected(m)` | a rejection read as `Failed`: `a_rejection_and_a_load_failure` FAILED | same |
| T022 (red), T023 | FR-023 statement names the filter | 034's statement kept (`if false`): `the_statement_names_the_filter_while_one_is_active` FAILED | same |
| T022 (red), T023 | FR-016 no search beyond without a word | any typed text searches (`searchable = true \|\| …`): `an_incomplete_filtered_list_searches_beyond_it_with_the_filter` FAILED (`left: Some(4) right: None`: a search pending for `""`) | same |
| T024 (red), T025 | requests only from the named events; the filter files log nothing | `start_issue_search`'s filtered branch removed: `every_allowlist_entry_still_matches` FAILED; an `eprintln!("{search:?}")` in `shell/github_filter.rs`: `the_github_filter_files_log_and_notify_nothing` FAILED | named-events 4 passed |
| T026 | panel frame `filters_panel` shared by the Jira and GitHub buttons | refactor; `jira_filters_fit` unchanged | `layout_snapshot` 166 passed |
| T027 (red), T028 | Filters button, Clear filters, active line, six chips wrapping, footer reachable | first run on the first layout (button beside the search, as selector-ui §1): `issue_rows_show_all_text` ×2, `picker_row_tooltip_clears_its_row` ×10 and `containment` FAILED, the search's list narrowed by the button wrapped long titles past the eight-row list; and `github_filters_fit` FAILED (search placeholder not painted). Moved the button to the row heading the picker (ledger M2 note) | `layout_snapshot` 166 passed after re-recording `layout_snapshot.txt` (GitHub issue states + 2 new) |
| T029 | `examples/fake_gh.rs` | dev-only tool, no test; built by `clippy --all-targets` | — |
| T030 | user guide "Filtering GitHub issues" | docs | — |

## M2 review round 1 fixes (red run before the code: each test written and run first)

Red run: `cargo test -p micold-material --lib a_press_on_a_panel_field` and `cargo test -p micold-client --test github_filter_state` on the tree with the tests and no fix (only the no-behaviour seam `WorktreeForm::issue_search_retries`, so the tests compile).

| Finding | Behaviour | Red (before the code) | Green |
|---|---|---|---|
| A F1 | an edit to quotes alone drops a pending search | `an_edit_to_quotes_alone_drops_the_pending_search` FAILED (`left: Some(4) right: None`: the pending search stayed) | `!typed \|\| !searchable` → `Idle`, `searched` cleared |
| A F2 | Clear filters with text typed and 034's listing capped searches beyond it | `clear_filters_with_text_typed_searches_beyond_a_capped_default_list` FAILED (`left: None right: Some(5)`) | `take_listing` drops the provider/filter gate; `on_issues_loaded` already batches `search_after_pause` |
| A F3 | a refused search beyond says "GitHub did not accept the search query: …", no Retry | `a_refused_filtered_search_says_so_without_retry` FAILED (`Search beyond the loaded issues failed — Couldn't read issues: GitHub did not accept the search query. Bad query`) | `SearchState::Rejected(messages)`, `refusal_line`, error caption without Retry |
| Visual F1 (B4) | each field offers every label not in its own list; picking one the other holds moves it | `a_label_chosen_in_one_list_moves_when_chosen_in_the_other` FAILED (`blocked` not offered in Include); `the_choices_are_alphabetical_less_the_chosen` FAILED (`["blocked", "in-progress"]` vs `["blocked", "bug", "in-progress"]`) | `offered(list)` filters the field's own list only |
| Visual F5 | a labels failure is named for the labels | `a_choice_failure_shows_retry_on_that_field_only` FAILED (`"Couldn't read issues: HTTP 502: Bad Gateway"`) | `label_failure` drops 034's "Couldn't read issues" opening |
| Visual F4 (B13) | a press on a panel field takes the keyboard from the trigger (one focus ring; Enter no longer presses Filters) | `a_press_on_a_panel_field_takes_the_keyboard_from_the_trigger` FAILED (`left: [true, true, false] right: [false, true, false]`) | `cdk::picker` Menu: after a press, a focused panel control unfocuses the trigger |
| Visual F3 | the "labels not offered" note wraps under a heading | test written with the code; red by mutant (the `NotOffered` arm removed): see below | `LabelRow::summary` gives `NotOffered` "Labels not offered" |
| Visual F2 | `fake_gh` answers the listing before the labels; `FAKE_GH_MODE` modes | dev-only tool, no test | — |

Visual F3 mutant: with the `LabelText::NotOffered(_)` arm removed from `LabelRow::summary`, `the_note_on_labels_not_offered_has_a_heading_to_wrap_under` FAILED (github_filter.rs:591); restored, it passes.

## M2 review round 2 fixes (red run before the code)

| Finding | Behaviour | Red (before the code) | Green |
|---|---|---|---|
| A F1 (MINOR) | core names a GitHub failure's cause without 034's "Couldn't read issues" opening, for every variant | `causes_leave_out_the_read_issues_opening` (micold-core `github_classify`) failed to compile: `no method named cause found for enum IssueLoadError` | `IssueLoadError::github_cause`; `github_message` and it share `github_parts`; `label_failure` calls it (no prefix stripping); `github_classify` 8 passed, `github_filter_state` 26 passed |
| Visual F2 (MINOR) | after a pick the label field stays open (it keeps the keyboard, so it is drawn focused) | `a_picked_label_leaves_the_field_open_with_no_highlight` FAILED: "the field holds the keyboard, so it is drawn open" | `GithubSelector::choose` no longer closes the field |
| Visual F3 (MINOR) | a pick resets the field's highlight; typing then highlights no row | same test, and the added assertion in Jira's `the_keyboard_reaches_every_row_picks_with_enter_and_escape_keeps_the_choice` FAILED (`left: Some(1) right: None`) | `choose` (GitHub) and `restore_query` (Jira) set `highlight = None`; `jira_filters_picker` 52 passed |
| A F2 (MINOR) | doc/comment wording | prose only | — |

## M3 — remember the basic filters per project (T031–T035; red run before the code)

Every test was written first and run against a no-behaviour seam (so it compiles and fails on
behaviour, not on a missing name): `shell/remembered.rs` with `read` answering the default and
`remember`/`remove` answering `Ok(())` without touching the disk; `Outcome::GithubSelectorRemembered`
declared but never emitted; `github_filter::Msg::Restored` accepted and ignored;
`GithubSelector::chip_text` answering the name alone; `Capabilities::github_selector_dir` held but
read nowhere. Then the code.

| Task | Behaviour | Red (seam, before the code) | Green |
|---|---|---|---|
| T031 (`tests/github_selector_shell.rs`) | lenient read; one-field read-modify-write keeps another window's fields; per-project file; removal, missing = success, failure = error; owner-only; failed write names no value | `cargo test -p micold-client --test github_selector_shell`: 6 FAILED (`a_missing_or_unreadable_file_reads_as_the_default`, `each_change_rewrites_one_field_and_keeps_the_others`, `removing_deletes_the_file_and_a_missing_file_is_success`, `a_removal_that_fails_is_an_error`, `the_file_is_owner_only`, `a_failed_write_is_an_error_naming_no_value`); 1 passed (`each_project_has_its_own_file`, a guard: the default reads as the default) | 7 passed |
| T031 (`main_tests.rs::github_selector_remembered`) | open reads the active project's file whatever GitHub availability; a change is written and restored at the next open, Clear filters remembered; two windows: last write wins, an open form stays; failed write notifies "Couldn't remember the GitHub issue filter: <OS error>" without the value; forget deletes the file (missing = success, failure notified) | `cargo test -p micold-client --bin micold-ai-ide github_selector_remembered`: 5 FAILED (`opening_restores_…` left default, right exclude×2; `a_change_is_written_…`; `with_two_windows_…`; `a_failed_write_…`; `forgetting_the_project_…`); 1 passed (`another_project_or_an_unreadable_file_opens_on_the_default`, a guard) | 6 passed; bin 757 passed |
| T032 (`tests/github_filter_state.rs`) | restored F → one filtered listing with F, active line (AS1); B's own or default (AS2); `Remembered(Basic)` at each basic change incl. Clear filters (AS5) and never otherwise; restoring remembers nothing; a remembered label the repository lacks applies and its chip says "· not in this repository" | 5 FAILED (`reopening_applies_…`, `another_project_opens_…`, `each_basic_change_is_remembered_…` left `[]`, `restoring_remembers_nothing`, `a_remembered_label_the_repository_lacks_…`); `a_label_in_another_case_or_beyond_a_partial_list_is_not_called_missing` passed on the seam (a guard against over-flagging) | 32 passed |
| T033 | `shell/remembered.rs` extracted; `jira.rs` `read_query`/`remember_query`/`remove_query` (and `remove_token`) call into it | refactor: `jira_connection_shell` 13 passed before, unchanged file | 13 passed |
| T034 | outcome, partition, reducer emits, `from_remembered`, shell read/write/delete | covered by the T031/T032 reds above | above |
| T035 | chip text drawn; user guide paragraph | `chip_text` red above; docs | — |

## M4 — filter by state, with a Closed chip (T036–T039; red run before the code)

Seam first, so the tests fail on behaviour: `Msg::StateChosen(IssueState)` accepted and ignored,
`GithubSelector::state_choices` answering `[]`, `issue_row` unchanged.

| Task | Behaviour | Red (seam, before the code) | Green |
|---|---|---|---|
| T036 (`tests/github_filter_state.rs`) | State row Open/Closed/All, Open default (AS1); Closed → one `is:closed` listing, all rows closed (AS2); All → no state term, Open → 034's listing; a closed issue picks like an open one (AS3, FR-017); `Remembered(Basic)` per state change, none for the chosen state; remembered Closed reopens with one `is:closed` listing | `cargo test -p micold-client --test github_filter_state`: 6 FAILED (`the_state_row_offers_…`, `closed_lists_the_closed_issues_with_one_listing`, `all_sends_no_state_term_…`, `a_closed_issue_fills_the_form_…`, `each_state_change_is_remembered`, `a_remembered_closed_state_reopens_…`); 32 earlier passed | 38 passed |
| T037 (`gates/issue_rows_show_all_text.rs`, state `add-worktree-dialog-issue-list-closed`) | every closed row paints its "Closed" chip (also a closed issue whose listing said "In Progress"); a closed Jira-keyed row shows "Closed" over the board's "In Progress", an open one keeps the board's | `--test layout_snapshot issue_rows_show_all_text`: 3 FAILED (`a_closed_row_shows_closed_not_a_stale_status`, both `every_issue_row_shows_all_of_its_text*`: no "Closed" painted) | 7 passed |
| T038 | reducer arm, State row, closed chip ahead of the board | above | above |
| T039 | docs | — | — |

`layout_snapshot.txt` re-recorded: the new covered state, and the Filters panel is 40px taller (the State row).

### M4 review round 1 fixes (MINORs, test first)

| Finding | Behaviour | Red (before the code) | Green |
|---|---|---|---|
| A F1 (`tests/github_filter_state.rs::the_cap_caption_names_the_chosen_state`) | the cap caption's count is "open issues", "closed issues" or "issues" by the chosen state | `cargo test -p micold-client --test github_filter_state -- the_cap`: FAILED (Closed said "1,234 open issues") | 40 passed |
| Visual F2 (`the_pre_source_hint_names_the_remembered_state`) | the statement before the source is chosen reads open / closed / open and closed issues from the remembered state | FAILED (Closed said "reads open issues") | 40 passed |
| Visual F1 (`ui/github_filter.rs::clear_filters_coming_and_going_keeps_the_filters_control_in_its_slot`) | the row ahead of the Filters control has the same slots with and without **Clear filters**, so focus (kept by tree position) stays on the chosen state chip or label field; label picks that toggle Clear filters shared the defect and share the fix | `cargo test -p micold-client --lib clear_filters_coming`: FAILED (2 slots against 3) | passed |

B F1 (Verify split into two commands) and B F2 (docs re-wrap) are prose only. `fake_gh` now answers the
single-issue details query (checked by parsing its answer with `parse_details`).

## M5 — advanced GitHub search query, with its errors (T040–T049)

| Task | Behaviour | Red (seam, before the code) | Green |
|---|---|---|---|
| T041 + T046 (`tests/github_filter_state.rs`) | switch to Advanced sends nothing, keeps search and list (AS1); draft edits never send (FR-010); Run → one listing `repo:o/r is:issue (<condition>) sort:…`, draft cleaned, `listed_for = Advanced` (AS2); `sort:created-asc` kept, GitHub's order, cap caption "Showing the first …" (AS3); closed issues listed and marked (AS4); empty → "No issue matches this query." (AS5); back to Basic reloads only after a query's list, draft kept (AS6); a switch drops the answer in flight (to Advanced: the prompt, switch stays); search beyond carries the query; Jira and back re-runs the last run query, not the draft (none run: nothing sent); refusals `repo:other/x`, `(a`, `label:"x`, 1,001 chars → `Refused(r)`, nothing awaited (US4 AS5); `Rejected([m1, m2])` kept with the draft, a new run replaces it (AS1, AS2); `Load(Offline)` → Failed, Retry runs the last run query (AS3); `Rejected([])` (AS4) | `cargo test -p micold-client --test github_filter_state` with `Msg::{ModeChanged, DraftChanged, Run}` and `IssueList::Refused` as no-op stubs: 14 FAILED, 40 passed | 54 passed |
| T042, T047 (reducer) | `change_github_mode`, `run_github_query`, `github_source_shown`, `listing_mode` (the list's own mode drives `active_filter`, `listed_state`), `sorted_by_query` | above | above |
| T043 (`gates/github_filters_fit.rs::the_query_row_and_the_cap_caption_fit_the_dialog`, `gates/panel_placement.rs`, covered state `add-worktree-dialog-github-advanced`) | advanced mode: no Filters / Clear filters; Basic, Advanced, the query and **Run** whole inside the dialog; query and Run one row, the query one line; the switch heads the picker; the sorted cap caption "Showing the first 2 of 1,234 issues…" whole under the query row; Create and Cancel in the window under it | `cargo test -p micold-client --test layout_snapshot -- github closed`: 1 FAILED (`"Filters" shows in advanced mode`), 6 passed | 7 passed (the one-line bound loosened to 1.5 lines of the button's text: the field's text is a size up, 20.8 against 20.0) |
| T044, T047 (UI) | `mode_switch(mode, on_change, r)` (T040; the chips are now keyboard stops for both sources), `query_row` (TextField "Query" `.on_submit(Run)` + **Run**), the prompt in place of the list before a run, the refusal error-tinted | above | above |
| Enter in the Query input (`micold-material` `text_field::enter_in_a_focused_field_with_on_submit_is_captured`) | Enter with `on_submit` publishes once and is captured, so it reaches no button behind the scrim | guard, not red: iced's `text_input` already captures it (green on first run) | 3 passed |
| Scoped gate 1 (`tests/field_focus_call_sites.rs`) | the Query input reports its focus (`FieldId::AddWorktreeGithubQuery`, `.track_focus`) | `every_input_in_the_application_reports_its_focus` FAILED (`ui/github_filter.rs:181 TextField::new`) | passed in scoped gate 2 |

Also fixed: `gates/issue_rows_show_all_text.rs` (M4's `a_closed_row_shows_closed_not_a_stale_status`) did not compile against 1504's `StatusBoard::begin(status, summary)` / `KeyedIssue` on this base.
