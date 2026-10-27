# Data Model: Filter the GitHub issue selector by search query, label and state

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md)

Core types are in the new module `micold_core::github_filter`
(`crates/micold-core/src/github_filter.rs`); client types in the new
`micold_client::features::worktree_form::github_filter`
(`crates/micold-client/src/features/worktree_form/github_filter.rs`). Only `RememberedSelector`
and the value types it holds derive `Serialize`/`Deserialize`. Every type that holds a filter value
or query text has a hand-written `Debug` that prints `<redacted>` for it (FR-024, research R15).

Sections marked **US6** are built only in the last milestone and are separable: no earlier type,
function or test names them (plan §US6 separability).

## 1. Basic filter values (core)

| Type | Shape | Rule |
|---|---|---|
| `LabelName` | `struct(String)` | Only from `LabelName::parse(&str) -> Option<LabelName>`: non-empty, no `"`, no control character (FR-006, research R6). Compared exactly (GitHub label names are case-insensitive on github.com, but the choice list and the remembered value hold GitHub's own spelling). |
| `IssueState` | `enum { Open, Closed, All }` | `Default = Open`; serde lowercase. |
| **US6** `Login` | `struct(String)` | Only from `Login::parse`: 1–39 ASCII letters, digits and single inner hyphens. |
| **US6** `Assignee` | `enum { Me, Nobody, User(Login) }` | |
| **US6** `Author` | `enum { Me, User(Login) }` | |
| **US6** `MilestoneTitle` | `struct(String)` | Only from `MilestoneTitle::parse`: non-empty, no `"`, no control character. |
| **US6** `Milestone` | `enum { Nobody, Titled(MilestoneTitle) }` | |

## 2. `BasicFilters` (core) — the basic filter set (FR-003, FR-004)

| Field | Type | Rule |
|---|---|---|
| `include` | `Vec<LabelName>` | Chosen order, no duplicates. |
| `exclude` | `Vec<LabelName>` | Chosen order, no duplicates, disjoint from `include`. |
| `state` | `IssueState` | |
| **US6** `assignee` | `Option<Assignee>` | `#[serde(default)]` |
| **US6** `author` | `Option<Author>` | `#[serde(default)]` |
| **US6** `milestone` | `Option<Milestone>` | `#[serde(default)]` |

- Fields private; changed only through `include_label`, `exclude_label`, `remove_label`,
  `set_state`, `cleared` (and US6 `set_assignee`/`set_author`/`set_milestone`). `include_label(l)`
  removes `l` from `exclude` and vice versa (spec Edge "Same label included and excluded"), so the
  disjointness holds by construction (Principle V).
- `is_default(&self) -> bool`: no label (and US6: no assignee, author, milestone) and state `Open`.
- `chosen_count(&self) -> usize`: the number the **Filters** button shows: one per label, one for
  a non-`Open` state (US6: one per set field).
- `described(&self) -> Option<String>`: the active line (FR-007), `None` when default. Parts in
  this order, joined with `; `: `Labels bug, ui`; `Excluding labels in-progress, blocked`;
  `Closed issues` / `Open and closed issues`; US6 `Assigned to me` / `No assignee` / `Assigned to
  <login>`, `By me` / `By <login>`, `Milestone <title>` / `No milestone`.
- Serde: lenient read — an entry that fails its `parse` is dropped, a duplicate kept once, a label
  in both lists kept in `include` only.

## 3. `UserQuery` and `QueryRefusal` (core) — the advanced query (FR-010, FR-012)

| Field | Type | Rule |
|---|---|---|
| `text` | `String` | Cleaned (`query_text::clean`); ≤ 1,000 `char`s. |
| `sorts` | `Vec<Range<usize>>` | Byte ranges of each `sort:` token outside quoted strings. |

- Only from `UserQuery::parse(raw) -> Result<UserQuery, QueryRefusal>` (research R5).
- `condition(&self) -> String`: `text` without the sort tokens, spaces collapsed, trimmed.
- `sorts(&self) -> Vec<&str>`: the sort tokens as typed, in order.
- `as_str(&self) -> &str`: what runs, what the input shows after a run, what is remembered.
- The empty text parses (Edge "Empty advanced query").

```rust
pub enum QueryRefusal { TooLong, UnclosedQuote, ScopeQualifier, UnbalancedParentheses }
```

`message(&self) -> &'static str` (never contains the query, FR-024):

| Variant | Text |
|---|---|
| `TooLong` | "The query is longer than 1,000 characters. Shorten it and run it again." |
| `UnclosedQuote` | "The query has a quote that is not closed. Close it and run it again." |
| `ScopeQualifier` | "The query names a repository, organization or user. The selector already searches this repository's issues; remove repo:, org:, user: or owner: and run it again." |
| `UnbalancedParentheses` | "The query's parentheses do not match. Fix them and run it again." |

## 4. `GithubFilter` and `ScopedSearch` (core) — what a request filters by (R2, R4)

```rust
pub enum GithubFilter { Basic(BasicFilters), Advanced(UserQuery) }
```

- `is_default(&self)`: `Basic(f)` with `f.is_default()`. A default filter is never turned into a
  `ScopedSearch`: the shell takes today's path (FR-009).
- `ScopedSearch { text: String }`: only from `ScopedSearch::listing(repo, &filter)` and
  `ScopedSearch::beyond(repo, &filter, typed_text) -> Option<ScopedSearch>` (research R4 texts;
  the typed text enters only as quoted words with `"` and `\` removed, `None` when no word is left). `as_str()` is the `q`
  sent. `Debug` redacted.

## 5. `FilterFailure` (core) — why a filtered request listed nothing (FR-018, FR-019)

```rust
pub enum FilterFailure { Rejected(Vec<String>), Load(IssueLoadError) }
```

`Rejected(vec![])` renders as `NOT_ACCEPTED`: "GitHub did not accept the search query." (US4 AS4).

## 6. Choices (core)

| Type | Shape | Rule |
|---|---|---|
| `LabelChoices` | `{ labels: Vec<LabelName>, not_offered: usize, partial: bool }` | From the label pages: names failing `LabelName::parse` counted in `not_offered`; sorted case-insensitively, de-duplicated; `partial` when 1,000 were held and a cursor remained. |
| **US6** `MilestoneChoices` | `{ milestones: Vec<MilestoneTitle>, not_offered: usize }` | Open milestones, sorted by title. |
| **US6** `Person` | `{ login: Login, name: Option<String> }` | `Debug` redacted. Entries whose login fails `Login::parse` are skipped. |

## 7. `RememberedSelector` (core) — per project (FR-021)

| Field | Type | Rule |
|---|---|---|
| `mode` | `QueryMode` (reused from `jira_query`, research R12) | Default `Basic`. |
| `basic` | `BasicFilters` | Default: the default filter. |
| `query` | `String` | A `UserQuery::as_str()`; empty when none ran or the empty query ran. |
| `ran` | `bool` | Whether any query ever ran on the project (FR-013: an empty query that ran re-runs; none ever run shows the prompt). |

- `from_json(&[u8]) -> RememberedSelector`: lenient (unparseable → default; invalid values dropped
  as §2; a query `UserQuery::parse` refuses → `query = ""`, `ran = false`).
- `to_json(&self) -> Vec<u8>`; `with_change(self, &SelectorChange) -> Self`.
- `SelectorChange { Mode(QueryMode), Basic(BasicFilters), Query(String) }` — one field per write
  (research R11). `Query` sets `ran = true`.
- File: `store::github_selector_path(dir, project)` = `<dir>/<project_id_of(project)>.selector.json`,
  `dir` = `store::github_selector_dir()` = `paths::data_subdir("github")`.

## 8. `Issue` (core, existing) — one addition

| Field | Type | Rule |
|---|---|---|
| `closed` | `bool` | `false` for every listed default issue and every Jira issue; set from a search node's `state == "CLOSED"`. Builder `Issue::closed(self, bool)`, reader `is_closed()`. Takes no part in matching or ordering. |

`ui::worktree_form::issue_row` draws a closed GitHub issue with the status chip `TicketStatus {
name: "Closed", category: Some(StatusCategory::Done) }` (research R13), ahead of any status-board
entry for it, not stale (selector-ui §3).

## 9. `IssueList` (client, existing) — two variants

| Variant | Meaning |
|---|---|
| `Refused(QueryRefusal)` | The advanced query was not sent (FR-012); shown in place of the list. |
| `Rejected(Vec<String>)` | GitHub refused the filter or query (FR-018); its messages in place of the list. |

Both are reset like `Failed`. `held()` is empty for both.

## 10. `GithubSelector` (client) — the selector's state, one field of `WorktreeForm`

| Field | Type | Rule |
|---|---|---|
| `mode` | `QueryMode` | Changed only by the switch. |
| `basic` | `BasicFilters` | The basic filters. |
| `draft` | `String` | The query input as typed; starts as `last_run` when the form opens. |
| `last_run` | `Option<UserQuery>` | `None` when no query ever ran on the project. |
| `listed_for` | `Option<QueryMode>` | Which mode's filter produced the list shown (or in flight); decides whether switching to basic reloads (research R8). |
| `labels` | `ChoiceLoad<LabelChoices>` | Reuses 1185's `ChoiceLoad<T>` (`features::worktree_form::jira_filters`), its `start`/`answer` made `pub(super)`. |
| `include_field`, `exclude_field` | `FilterField` | Reuses 1185's typeahead field state. |
| `panel_open` | `bool` | |
| **US6** `milestones` | `ChoiceLoad<MilestoneChoices>` | |
| **US6** `people` | `ChoiceLoad<Person>` + `people_pending: Option<u64>` | Debounced search (1185). |
| **US6** `assignee_field`, `author_field`, `milestone_field` | `FilterField` | |

- `active_filter(&self) -> GithubFilter`: `Basic(basic.clone())` in basic mode; in advanced mode
  `Advanced(last_run)` when one ran, else none (the prompt, nothing sent).
- `from_remembered(RememberedSelector) -> GithubSelector` at form open.

### Transitions (reducer `features::worktree_form::github_filter::update`)

| Event | Effect |
|---|---|
| Form opens with `RememberedSelector` R | `GithubSelector::from_remembered(R)`; nothing loads until the GitHub source shows. |
| GitHub source shown (034's load point) | Basic: load with `basic` (default → today's load). Advanced: `last_run` some → run it once (FR-013); none → prompt, `issues = NotRequested`. |
| Basic filter chosen / removed / state set / **Clear filters** | Update `basic`; bump `issue_request_seq`; `issues = Loading`; `listed_for = Basic`; outcome `GithubSelectorRemembered(Basic)` (FR-008, FR-021). |
| Switch to Advanced | `mode = Advanced`; no load (FR-002); outcome `Remembered(Mode)`. |
| Switch to Basic | `mode = Basic`; if `listed_for == Some(Advanced)` → bump seq, load `basic`; outcome `Remembered(Mode)`. |
| Draft edited | `draft = text`; nothing else (FR-010). |
| Run (Enter / Run) | `UserQuery::parse(draft)`: `Err(r)` → bump seq, `issues = Refused(r)` (nothing sent); `Ok(q)` → bump seq, `draft = q.as_str()`, `last_run = Some(q)`, `issues = Loading`, `listed_for = Advanced`, outcome `Remembered(Query)`. |
| Filtered load answered (seq matches) | `Ok` → `Loaded` (descriptions done, R9); `Err(Rejected(m))` → `Rejected(m)`; `Err(Load(e))` → `Failed { e }`. Stale seq → dropped. |
| Retry | Same filter, new seq (FR-019). |
| Source → Jira, form closed, GitHub issues off | In-flight result discarded by the seq bump 034 already does; `GithubSelector` kept while the form is open (spec Edge "Switching source to Jira and back"). |
| Label field focused first time | `labels = Loading { seq }` → shell requests choices. |

Outcomes `Outcome::GithubSelectorRemembered { project, change: SelectorChange }` are performed by
the shell (`shell::issues::remember_github_selector`), as 1186's `JiraQueryRemembered`.
