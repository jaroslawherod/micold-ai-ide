# Contract: source, requests and remembered state

Research R2, R3, R7, R10, R11, R15.

## 1. `IssueSource` additions (`crates/micold-core/src/github.rs`)

Each has a default body answering `Err(Load(Other("not supported")))` (or `Err(Other(..))`), so no
other implementor changes. `GhCli` and `FakeIssueSource` implement all.

| Method | Request | Parser |
|---|---|---|
| `search_page(&self, repo, &ScopedSearch, cursor) -> Result<IssuePage, FilterFailure>` | `gh api graphql --hostname github.com -f query=<SEARCH_PAGE_QUERY> -f q=<scoped> [-f cursor=<c>]` | `parse_search_page` |
| `search_filtered(&self, repo, &ScopedSearch) -> Result<Vec<Issue>, FilterFailure>` | `-f query=<SEARCH_FILTERED_QUERY> -f q=<scoped beyond>` | `parse_search_page(..).map(|p| p.issues)` |
| `labels_page(&self, repo, cursor) -> Result<NamePage, IssueLoadError>` | `-f query=<LABELS_QUERY> -f owner= -f name= [-f cursor=]` | `parse_names_page(["repository","labels"])` |
| **US6** `milestones_page(&self, repo, cursor)` | `MILESTONES_QUERY` (states: OPEN) | `parse_names_page(["repository","milestones"])` (`title`) |
| **US6** `assignable_users(&self, repo, text) -> Result<Vec<Person>, IssueLoadError>` | `ASSIGNABLE_USERS_QUERY`, `-f q=<text>` | `parse_people` |

`NamePage { names: Vec<String>, next_cursor: Option<String> }`; `load_label_choices(source, repo)`
pages up to 10 pages and builds `LabelChoices` (data-model §6). Every variable is `-f` (raw
string), as 034. `answer_of` is reused: a `FilterFailure::Rejected` from the parser stands
whatever the exit code; a parser `Load(Other)` on a non-zero exit is classified from stderr.

`FakeIssueSource` gains `with_search_pages(Vec<Result<IssuePage, FilterFailure>>)`,
`with_search_filtered(..)`, `with_labels(..)` (US6: `with_milestones`, `with_people`), and records
every `ScopedSearch` text it was asked with (`asked_searches()`), so client tests assert what was
sent.

## 2. What leaves the machine (FR-023, FR-024)

| Request | Carries |
|---|---|
| Default listing, description pass, default search beyond | exactly 034's arguments (FR-009; `tests/github_privacy.rs` unchanged and green) |
| Filtered listing | `q` = scoped listing text (repository, filter) |
| Filtered search beyond | `q` = scoped beyond text (repository, filter, typed text) |
| Labels, US6 milestones | owner, name |
| US6 people | owner, name, typed text |

The picker's statement (`WorktreeForm::issue_notice`) with a non-default filter active:
"Reads issues of <repo> from GitHub, sending the filter and the search text." Default: 034's
"Reads open issues of <repo> from GitHub." unchanged.

No `tracing::`/`log::` macro and no `notify_*` call in `github_filter.rs` (core and client),
`shell/github_filter.rs` or `ui/github_filter.rs` takes a draft, `UserQuery`, `ScopedSearch`,
`BasicFilters` or label value (source scan, `issues_are_requested_only_on_named_events.rs`
extended).

## 3. When requests are made (FR-005, FR-013, FR-025)

| Event | Request |
|---|---|
| GitHub source shown, basic, default filter | 034's listing (unchanged) |
| GitHub source shown, basic, non-default | one filtered listing |
| GitHub source shown, advanced, a query ran before on this project | one filtered listing of it |
| GitHub source shown, advanced, none ever ran | none (prompt) |
| Basic filter change | one filtered (or default) listing |
| Run | none when refused; else one filtered listing |
| Switch to Advanced | none |
| Switch to Basic after a run | one listing with the basic filters |
| Typing search text with an incomplete filtered list | one filtered search after the 300 ms debounce (034 R9) |
| First focus of a label field | one label choice load (up to 10 pages) |
| US6: first focus of milestone; typing in assignee/author | one milestone load; one debounced people search |
| GitHub issues off (1102) | none of the above; no filter control drawn |

## 4. Shell (`crates/micold-client/src/shell/github_filter.rs`, new)

- `start_filtered_load(app, seq, filter)`: locates `gh` exactly as `start_issue_load` does (the
  located-`gh` closure is factored out of `start_issue_load`, not copied), runs `load_filtered` on a
  blocking thread, answers `FormMsg::GithubFilter(Msg::Loaded { seq, result })`.
  `shell::issues::start_issue_load` branches to it when the form's active GitHub filter is not
  default; `start_issue_search` branches to `search_filtered` likewise.
- `start_label_choices(app, seq)`, US6 `start_milestones`, `start_people_search`: same pattern.
- `remember_github_selector(app, outcomes)`: for each `Outcome::GithubSelectorRemembered { project,
  change }`, `shell::remembered::remember(dir, file_name, change)`; failure →
  notification "Couldn't remember the GitHub issue filter: <OS error>" (never the value, FR-024).
- `on_form_opened` (existing, `shell/issues.rs`) also reads the selector file for the active
  project and hands `RememberedSelector` to the form, whatever the GitHub availability.

## 5. Storage (FR-021, FR-022)

| Item | Value |
|---|---|
| Directory | `store::github_selector_dir()` = `paths::data_subdir("github")` (new) |
| File | `store::github_selector_path(dir, project)` = `<dir>/<project_id_of(project)>.selector.json` (new) |
| Write | `micold_core::owner_only::write` (temp file + rename), read-modify-write of one `SelectorChange` |
| Read | at form open only; missing or unreadable → default (Edge "Remembered state unreadable") |
| Kept | when GitHub issues are switched off, the source is Jira, the repository changes |
| Deleted | in `shell::daemon_sync::on_project_forget_confirmed`, beside the pane-layout cleanup; a missing file is success, a failure a notification |

`shell::remembered` (new): `read<T: DeserializeOwned + Default>(path) -> T` (via the type's lenient
`from_json`), `remember(dir, file_name, apply: impl FnOnce(T) -> T)`, `remove(path)`. `shell/jira.rs`'s
`read_query`/`remember_query`/`remove_query` become one-line calls into it (1186 behaviour and
tests unchanged).
