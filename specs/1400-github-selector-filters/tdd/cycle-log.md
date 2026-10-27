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
