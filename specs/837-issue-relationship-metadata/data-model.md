# Data model: issue details (837)

All types are in `micold_core::issue_details` unless a section says otherwise. None derives
`Serialize` (research R9).

## 1. `PlainText`

Text from GitHub after neutralising (FR-013). Its only constructor is `PlainText::new(raw: &str)`,
which drops control, zero-width and direction-override characters and keeps the joiners between
visible characters (research R8). `as_str()`, `Display`. An all-invisible input gives an empty
`PlainText`. Callers treat that as absent: a label, login or field value that is empty after
neutralising is left out.

## 2. `Reference`

```rust
enum Reference { Here(u64), Other { owner: String, name: String, number: u64 } }
```

- `Here` when the node's `repository.nameWithOwner` equals the picked issue's repository, ignoring
  ASCII case as GitHub does. Otherwise `Other`.
- `Display`: `#12`, or `owner/name#12` (spec Edge Cases, FR-002).

## 3. Related items

```rust
struct RelatedIssue { reference: Reference, title: PlainText, open: bool }
struct RelatedPr    { reference: Reference, title: PlainText, state: PrListState }
enum PrListState { Open, Draft, Merged, Closed }
```

`PrListState` mapping: `merged` gives `Merged`; `state == CLOSED` gives `Closed`; `isDraft` gives
`Draft`; anything else is `Open`. It is separate from `pull_request::PrState`, which carries check
status that the details do not read.

## 4. `RelatedList<T>`

```rust
struct RelatedList<T> { shown: Vec<T>, total: u32, hidden: u32, total_is_lower_bound: bool }
```

Invariants (FR-008):

- `shown.len() <= 20`. The constructor truncates and keeps GitHub's order.
- `total >= shown.len() + hidden`.
- `more() = total - shown.len()`.
- `total_is_lower_bound` is set only on the linked-PR list (research R4).
- `is_empty()` is `total == 0`.

## 5. `IssueDetails`

```rust
enum Kind<T> { Read(T), NotReadable(String /* plain reason */) }

struct IssueDetails {
    number: u64,
    blocked_by: Kind<Blockers>,               // Blockers { list: RelatedList<RelatedIssue>, open: u32 }
    blocking: Kind<RelatedList<RelatedIssue>>,
    parent: Kind<Option<RelatedIssue>>,
    sub_issues: Kind<SubIssues>,              // SubIssues { list: RelatedList<RelatedIssue>, closed: u32 }
    labels: Kind<Vec<PlainText>>,
    assignees: Kind<Vec<PlainText>>,          // logins
    milestone: Kind<Option<PlainText>>,
    projects: Kind<Vec<ProjectMembership>>,
    closing_prs: Kind<RelatedList<RelatedPr>>,
    linked_prs: Kind<RelatedList<RelatedPr>>, // closing PRs removed (spec Edge Cases)
}

struct ProjectMembership { name: PlainText, fields: Vec<(PlainText, PlainText)> } // non-empty values only
```

- **`open`** (blockers): `issueDependenciesSummary.blockedBy`, so it counts hidden and unlisted
  blockers too.
- **`closed`** (sub-issues): `subIssuesSummary.completed`. The sub-issue total is
  `subIssuesSummary.total`.
- **`is_blocked()`**: `matches!(blocked_by, Kind::Read(b) if b.open > 0)` (FR-003, SC-003). A
  not-readable blocker kind is never blocked, and is reported as not readable.
- **`is_present(kind)`**: `Read` with something in it, or `NotReadable`. Only present kinds are
  output (FR-004, FR-005, FR-007).
- **Labels and assignees**: capped at 20 with a count of the rest (`labels.totalCount`), like the
  related lists.
- **Projects**: capped at 20. A project with no non-empty field value keeps its name only (spec
  Edge Cases).
- **`Debug`**: hand-written. It prints the number and per-kind counts, never titles or logins.

## 6. Form state (`micold_client::features::worktree_form`)

```rust
enum IssueDetailsState {
    Idle,
    Loading { seq: u64, number: u64 },
    Loaded  { number: u64, details: IssueDetails },
    Failed  { number: u64, message: String },
}
```

Transitions:

| From | Event | To |
|---|---|---|
| any | GitHub issue picked (`set_picked_issue(Some(n))`, GitHub provider) | `Loading { seq: ++details_seq, number: n }` |
| any | pick cleared, source left, Jira pick | `Idle`, and `details_seq` is bumped so an answer in flight cannot match |
| `Loading { s, n }` | `IssueDetailsLoaded { seq: s, number: n, Ok(d) }` | `Loaded { n, d }` |
| `Loading { s, n }` | `IssueDetailsLoaded { seq: s, number: n, Err(e) }` | `Failed { n, e.message(repo) }` |
| other | `IssueDetailsLoaded` not matching | unchanged (dropped, FR-012) |
| form closed | — | the state is gone with the form, so any answer is dropped |

Create (`submitted`) reads none of this (FR-006).

## 7. Display line (core, for the form)

```rust
struct DetailLine { label: &'static str, text: String, warning: bool }
```

`IssueDetails::display_lines() -> Vec<DetailLine>`, in a fixed order: Blocked by, Blocking, Parent,
Sub-issues, Closing pull requests, Linked pull requests, Assignees, Milestone, Projects. There is no
Labels line: the picked issue's row already shows its labels (034/038), and US2-2 does not list them.
Contract [form-details.md](contracts/form-details.md) §3 gives each line's text. `warning` is true
only on the Blocked-by line when `is_blocked()`.
