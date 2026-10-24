# Data Model: Advanced JQL in the Jira Issue Selector

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md)

All core types are in the new module `micold_core::jira_query`
(`crates/micold-core/src/jira_query.rs`); client types in the new
`micold_client::features::worktree_form::jira_query` (`crates/micold-client/src/features/worktree_form/jira_query.rs`).
None derives `Serialize` except `RememberedQuery`, and no issue type gains it
(`crates/micold-client/tests/jira_is_requested_only_on_named_events.rs`).

## 1. `UserQuery` (core) — the parsed, cleaned user query

| Field | Type | Rule |
|---|---|---|
| `text` | `String` | Cleaned (R3): `\n` `\r` `\t` → space, other control characters removed, trimmed. ≤ 2,000 `char`s. |
| `sort_at` | `Option<usize>` | Byte offset in `text` of the first sort clause outside quoted strings (R2); `None` without one. |

- Constructed only by `UserQuery::parse(raw: &str) -> Result<UserQuery, QueryRefusal>`; no public
  constructor or field, so an unclosed quote, an unbalanced parenthesis or an over-long text cannot
  be held (Principle V).
- `condition(&self) -> &str`: `text[..sort_at]` trimmed (all of `text` without a sort clause).
- `sort_clause(&self) -> Option<&str>`: `text[sort_at..]` trimmed, kept as typed (FR-005).
- `as_str(&self) -> &str`: `text`, what the input shows after a run and what is remembered.
- The empty text parses (Edge "Empty query").

## 2. `QueryRefusal` (core) — why the app did not send a query (FR-007)

```rust
pub enum QueryRefusal { UnclosedQuote, UnbalancedParentheses, TooLong }
```

`message(&self) -> &'static str` gives the selector's text ([contracts/jql-rules.md §5](./contracts/jql-rules.md#5-texts)).
Precedence when several hold: `TooLong` (checked first, before the scan), then `UnclosedQuote`,
then `UnbalancedParentheses`.

## 3. `ScopedQuery` (core) — the only JQL advanced mode sends (FR-005)

| Field | Type | Rule |
|---|---|---|
| `jql` | `String` | `project = "<KEY>" AND (<condition>) <sort>`; with an empty condition `project = "<KEY>" <sort>`; `<sort>` is the user's sort clause or `ORDER BY updated DESC`. |

- Constructed only by `ScopedQuery::new(key: &ProjectKey, query: &UserQuery)`; `as_str()`.
- `ProjectKey` (`crates/micold-core/src/jira.rs`) is validated, so the key cannot carry a quote.
- Equality is the JQL's, so Retry (FR-011) re-sends an equal value.

## 4. `QueryFailure` (core) — why a sent query listed nothing

```rust
pub enum QueryFailure {
    /// Jira answered 400: its messages (R4), possibly empty.
    Rejected(Vec<String>),
    /// Any other failure, classified as basic mode's (533 FR-018).
    Load(IssueLoadError),
}
```

`Rejected(vec![])` shows the plain "not valid JQL" text (FR-010, US2 AS4). `Load` renders through
`IssueLoadError::message(NoticeSource::Jira{..})`, so its notices are basic mode's word for word
(FR-011).

## 5. `RememberedQuery` (core) — per project, on disk (FR-013, FR-014)

| Field | Type | Default |
|---|---|---|
| `mode` | `QueryMode` (`Basic` \| `Advanced`, serde `lowercase`) | `Basic` |
| `query` | `String` (a `UserQuery::as_str()`, never a draft) | `""` |

- File: `<data_dir>/jira/<project_id_of(project)>.query.json`, owner-only, beside
  `<project id>.token` ([contracts/selector-source-and-storage.md §4](./contracts/selector-source-and-storage.md#4-the-remembered-file)).
- `from_json(bytes) -> RememberedQuery` is lenient: unreadable JSON, an unknown mode or a `query`
  that `UserQuery::parse` refuses reads as the default (Edge "Remembered data unreadable").
- `with_mode(self, QueryMode)` and `with_query(self, &UserQuery)` change one field, for the
  read-modify-write.

## 6. `JiraQuery` (client) — the selector's advanced state, one field of `WorktreeForm`

| Field | Type | Meaning |
|---|---|---|
| `mode` | `QueryMode` | Shown mode. Set from the remembered file at open; changed by the switch. |
| `draft` | `String` | The input's text as typed (FR-003). Starts as `last_run`. |
| `last_run` | `String` | The project's last run query (from the file, then each run). |
| `run` | `QueryRun` | Where the advanced list is (below). |
| `highlight` | `Option<usize>` | Keyboard row, an index into the shown results. |
| `list_open` | `bool` | Whether the result list shows. |

`QueryRun`:

| Variant | Shown in place of the list |
|---|---|
| `Idle` | The prompt to type and run a query (FR-004a), or the previous result until a run (US1 AS1). |
| `Refused(QueryRefusal)` | The refusal text; nothing was sent (FR-007). |
| `Running { seq: u64, scoped: ScopedQuery }` | Progress "Running the query on *host*…". |
| `Failed { scoped: ScopedQuery, failure: QueryFailure }` | `Rejected`: the messages, one per line (FR-010); `Load`: the notice and **Retry** (FR-011). |
| `Loaded { scoped: ScopedQuery, listing: IssueListing }` | The rows in Jira's order (FR-006); empty → "No issue matches this query." (FR-012); `!complete` → the cap caption. |

The basic `IssueList`, `issue_query` and (with #1185) the filters on `WorktreeForm` are not
touched by any of these: switching back to basic shows them as they were (FR-003).

### Transitions

| Event | From → To | Notes |
|---|---|---|
| Form opens, file read (`Restored`) | any → `mode`, `draft = last_run = query`, `Idle` | Nothing is sent yet. |
| Jira source becomes shown (`SourceChanged(Issue)` with Jira, provider → Jira, or back from GitHub) in `Advanced` with non-empty `last_run` | → `Running { next seq }` | FR-004a, FR-003a; no basic load starts. |
| Same, `Advanced` with empty `last_run` | → `Idle` (prompt) | FR-004a. |
| `ModeChanged(Advanced)` | basic → advanced; `run` unchanged | No run (FR-004a); an in-flight basic load is discarded (seq bump, `issues` → `NotRequested`). Remember mode. |
| `ModeChanged(Basic)` | advanced → basic; `Running` → `Idle` | In-flight run discarded (seq bump). Basic list `NotRequested` → load starts (FR-003). Remember mode. |
| `DraftChanged(text)` | `draft = text` | Never runs (FR-004). |
| `Run` (Enter / Run control) | parse fails → `Refused`; else → `Running { next seq }` | `draft` and `last_run` become the cleaned text; remember query (also when empty: US3 AS4). |
| `Ran { seq, result }` with the awaited seq | `Running` → `Loaded` / `Failed` | Any other seq is dropped. |
| `Retry` | `Failed { scoped, Load(_) }` → `Running { next seq, scoped }` | Same scoped query (FR-011). |
| Source to GitHub | `Running` → `Idle`; `draft = last_run` | Mode kept (FR-003a); seq already bumped by `reset_issues`. |
| Integration off / connection removed / form closed | form state dropped or reset as basic's | File kept unless the connection is removed (FR-014). |
| `Picked(index)` | row `index` of `Loaded` | Fills the form exactly as `issue_picked` (FR-009). |

## 7. Messages and outcomes (client)

`worktree_form::Msg::JiraQuery(jira_query::Msg)` with
`jira_query::Msg = Restored(RememberedQuery) | ModeChanged(QueryMode) | DraftChanged(String) | Run
| Retry | Ran { seq, result: Result<IssueListing, QueryFailure> } | Focused | HighlightMoved(Direction)
| Dismissed | RowPicked(usize)`.

`features::Outcome::JiraQueryRemembered { project: PathBuf, change: RememberChange }` with
`RememberChange = Mode(QueryMode) | Query(String)`; the shell does the read-modify-write.
