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
