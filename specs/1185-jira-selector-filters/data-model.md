# Data model: Jira issue selector filters

All state is in memory, held by the open create-worktree form, and never serialised (FR-006,
FR-016). Core types live in `crates/micold-core/src/jira_filter.rs` (NEW); form state in
`crates/micold-client/src/features/worktree_form/jira_filters.rs` (NEW).

## 1. `AccountId` (core)
- A Jira Cloud account id: non-empty, no control characters. Built only by `parse_myself` /
  `parse_users` (or `AccountId::parse` in tests).
- `Debug` prints `AccountId(<redacted>)`. No `Serialize`.

## 2. `JiraUser` (core)
| Field | Type | Rule |
|---|---|---|
| `account` | `AccountId` | identifies the person (FR-004, FR-010) |
| `name` | `String` | display name; non-empty, no control characters |
| `is_me` | `bool` | the connected account's own user (`/myself`) |
- Equality by `account` only (two users sharing a display name stay distinct, US2 AS3).
- `Debug` prints `JiraUser(<redacted>)`. Shown in the UI as `name`, or "You (name)" when `is_me`.

## 3. `VersionChoice` (core)
| Field | Type | Rule |
|---|---|---|
| `name` | `String` | non-empty, no control characters |
| `released` | `bool` | |
- Archived versions never become a `VersionChoice`. Order: unreleased (Jira order), then released
  (Jira order reversed) (research R3).

## 4. Components and labels (core)
- Component choices: `Vec<String>` of names, Jira's order, duplicates and control-character names
  dropped.
- `LabelChoices { labels: Vec<String>, partial: bool }` from `label_choices(&IssueListing)`
  (research R5): sorted, unique, `partial` = listing incomplete.

## 5. `JiraFilters` (core)
| Field | Type |
|---|---|
| `reporter` | `Option<JiraUser>` |
| `label` | `Option<String>` |
| `component` | `Option<String>` |
| `fix_version` | `Option<String>` |
- At most one value per filter (FR-002). Fields private; setters `set_reporter`, `set_label`,
  `set_component`, `set_fix_version` take `Option<…>` and refuse (leave unchanged, return `false`)
  a value with a control character or an empty one; `clear_all()`.
- `is_empty()`, `active() -> Vec<(FilterKind, String)>` (what the chips and the FR-014 statement
  name), `clauses() -> Vec<String>` (research R1/R2).
- `Debug` derives through `JiraUser`'s redacted `Debug`.
- `FilterKind`: `Reporter | Label | Component | FixVersion` (fixed order for clauses and chips).

## 6. `ChoiceLoad<T>` (client, render-free)
```
NotRequested ──focus──▶ Loading{seq} ──ok──▶ Loaded(Vec<T>)
                              │
                              └──err──▶ Failed{error} ──Retry──▶ Loading{seq'}
```
- A result applies only when its `seq` equals the awaited one; otherwise it is dropped.
- `Loaded(vec![])` renders the "This project has no …" reason (edge case: empty choices).

## 7. `FilterState` (client, on `WorktreeForm` as `jira_filters`)
| Field | Type | Notes |
|---|---|---|
| `chosen` | `JiraFilters` | what the listing is filtered by |
| `components` | `ChoiceLoad<String>` | |
| `versions` | `ChoiceLoad<VersionChoice>` | |
| `me` | `ChoiceLoad<JiraUser>` | at most one user |
| `people` | `ChoiceLoad<JiraUser>` | last typed search; own user removed |
| `labels` | `LabelSource` | `Waiting { seq }` (the unfiltered load's seq) / `Failed` / `Ready(LabelChoices)`; a result of that seq fills `Ready` even when a filter change superseded it as the listing (FR-005) |
| `queries` | per-kind typed text | each field's own text |
| `matches`, `highlight`, `open` | per-kind | as the issue typeahead (`rank`, `clamp_highlight`); row 0 of every list is "Any <filter>" |
| `people_pending` | `Option<u64>` | the debounced reporter search awaiting its due time |
- Reset to `FilterState::default()` whenever the issue provider leaves Jira, the source leaves
  Issue, the Jira availability changes, or the form opens (FR-016).
- `filter_request_seq: u64` lives on `features::worktree_form::State`, beside
  `issue_request_seq`, so a late result never matches a new form's request.

## 8. Messages (`FilterMsg`, wrapped as `FormMsg::JiraFilter(FilterMsg)`)
`Focused(FilterKind)`, `QueryChanged(FilterKind, String)`, `HighlightMoved(FilterKind,
Direction)`, `Dismissed(FilterKind)`, `Picked(FilterKind, usize)` (row 0 = Any: clears), `Cleared(FilterKind)`,
`ClearedAll`, `ChoicesRetry(FilterKind)`, `PeopleSearchDue { seq }`,
`ComponentsLoaded { seq, result }`, `VersionsLoaded { seq, result }`, `MeLoaded { seq, result }`,
`PeopleLoaded { seq, result }`.
- `Picked`, `Cleared`, `ClearedAll` that change `chosen` bump `issue_request_seq` and set
  `IssueList::Loading { seq }` (research R6). Picking the value already chosen changes nothing.
