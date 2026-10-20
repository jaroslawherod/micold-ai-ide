# Contract: Jira status source

**Feature**: [spec.md](../spec.md) | Extends [533 jira-cloud-source](../../533-jira-issue-worktrees/contracts/jira-cloud-source.md) | [research R1, R2](../research.md)

## 1. Trait addition (`micold_core::jira`)

```rust
pub trait JiraSource: Send + Sync {
    // list_open, search_open unchanged
    /// The status of each of `keys` (at most 100) of the connected project. A key Jira does not
    /// return is absent from the answer (not found or not visible).
    fn statuses(&self, conn: &JiraConnection, token: &ApiToken, keys: &[String])
        -> Result<Vec<(String, Option<TicketStatus>)>, IssueLoadError>;
}
```
The trait method has a default body returning `Ok(vec![])`, so the `Recording` impl in
`main_tests.rs` still compiles; `Recording` is updated where a test needs statuses.
`FIELDS` becomes `[&str; 7]`.
`FakeJiraSource::with_statuses(Result<..>)` scripts answers; `status_calls()` records the key lists
(SC-003 asserts one call for 50 keys).

## 2. Picker requests

`FIELDS` gains `"status"` (list and search). `parse_page` sets `Issue::with_status`. JQL unchanged
(FR-011).

## 3. Status request

`POST {origin}/rest/api/3/search/jql`, body
`{"jql":"project = \"KEY\" AND key in (\"KEY-1\",\"KEY-2\") ORDER BY key ASC","fields":["status"],"maxResults":<n>}`.
- Keys are validated `<PROJECT>-<digits>` of the connected project and uppercased before use; others
  are dropped, so the JQL cannot carry a quote (pure `statuses_jql(project, keys)`).
- More than 100 keys: chunks of 100, in order; the first failure aborts the rest.
- Same headers, timeout, no redirects and token redaction as 533 §2.
- A `400` with unknown keys: bisect the chunk (depth cap 7); a key alone in a `400` is absent.
- No done filter: Done tickets are returned (FR-007).

## 4. Parsing (`parse_statuses`)

`{"issues":[{"key","fields":{"status":{"name","statusCategory":{"key"}}}}]}` -> `(key, status)`.
Missing or blank `name` -> `None`; unknown category key -> `category: None`. Non-JSON or no `issues`
-> `NotJiraCloud`, as `parse_page`.

## 5. Errors

Same `IssueLoadError` kinds. The board treats all as one failed refresh (data-model R5); messages
shown nowhere (FR-008), never contain token or site (FR-012).
