# Contract: the advanced request and the remembered file

**Feature**: [spec.md](../spec.md) | Research [R5](../research.md#r5--a-new-jirasource-method-and-the-400-body-in-the-adapter), [R7](../research.md#r7--where-the-remembered-query-and-mode-live), [R9](../research.md#r9--state-messages-and-requests-in-the-client) | Rules [jql-rules.md](./jql-rules.md)

## 1. The trait method (`crates/micold-core/src/jira.rs`, added at the end of `JiraSource`)

```rust
/// One page of the issues `jql` lists, in its order; `cursor` is Jira's nextPageToken. A 400 is
/// `QueryFailure::Rejected` with Jira's messages (feature 1186).
fn query_page(&self, _conn: &JiraConnection, _token: &ApiToken, _jql: &ScopedQuery,
              _cursor: Option<&str>) -> Result<IssuePage, QueryFailure> {
    Err(QueryFailure::Load(IssueLoadError::Other("this Jira source cannot run a query".into())))
}
```

`FakeJiraSource` adds `queries: Script<IssuePage>` answering `Result<IssuePage, QueryFailure>`,
`query_calls: Mutex<Vec<(String, Option<String>)>>` (JQL, cursor), `with_query(result)` and
`query_calls()`; an unscripted call answers `QueryFailure::Load(Other("FakeJiraSource: no query
scripted for this call"))`.

## 2. The request (`JiraCloud::query_page`, `crates/micold-client/src/shell/jira_http.rs`)

`POST {origin}/rest/api/3/search/jql`, body `{"jql": <scoped>, "fields": FIELDS, "maxResults":
100, "nextPageToken": <cursor>?}` — `request_body(scoped.as_str(), cursor, PAGE_SIZE)`. Headers,
10 s budget and no-redirect rule are `send`'s, unchanged (533 FR-020; this feature's FR-015 widens
only the JQL).

| Answer | Result |
|---|---|
| 2xx | `parse_page(body)` (`NotJiraCloud` → `Load(NotJiraCloud)`) |
| 400 | `Rejected(rejection_messages(body))`, body read up to 64 KiB |
| any other | `Load(classify(..))`, as basic mode |

`PostError::BadRequest` becomes `BadRequest(Vec<u8>)`; `post` and `read_chunk` match
`BadRequest(_)` and behave as today.

## 3. The run (`crates/micold-client/src/shell/jira_query.rs`, new)

`start_query(app, seq, scoped)`: `jira_request(app)` (the one gate, `shell/issues.rs`); on `Err(why)`
answer `Ran { seq, Err(Load(Other(why))) }`; else `request.run(|s, c, t| jira_query::load(s, c, t,
&scoped))` mapped to `Ran { seq, result }`. Started only when the reducer accepted `Run`, `Retry`, or
a Jira-source-shown event in advanced mode (FR-004a) — the shell compares `worktree_form::State::awaited_jira_query()` (new: the seq of
`JiraQuery.run == Running { seq, .. }`, `None` otherwise) before and after the message, as
`newly_awaited` does with `awaited_issue_load()`. Added to the allow-list of
`crates/micold-client/tests/issues_are_requested_only_on_named_events.rs`.

## 4. The remembered file

- Path: `jira_query_path(dir, project) = dir.join(format!("{}.query.json", project_id_of(project)))`,
  `dir = Capabilities::jira_token_dir()` (`<data_dir>/jira`). New helper in
  `crates/micold-core/src/store.rs` next to `jira_token_path`.
- Content: `{"mode":"advanced","query":"assignee = currentUser()"}` (data-model §5).
- Read: in `shell::issues::on_form_opened`, after `FormMsg::Opened`, when the Jira availability is
  `Connected`; a missing or unreadable file is `RememberedQuery::default()`; the reducer gets
  `JiraQuery(Restored(..))`. Synchronous: one file of at most ~2 KiB, read nowhere else.
- Write: on `Outcome::JiraQueryRemembered { project, change }`: read the file (lenient), apply
  `with_mode`/`with_query`, `owner_only::write` (temporary file + rename, so a reader never sees a
  half file). Two windows: the last write wins per field.
- Failure: a notification `Couldn't remember the Jira query: <io error>` — never the query (FR-016).
- Remove: `shell::jira::apply`, `ConnectionAction::Remove`, deletes the file after the token; a
  missing file is success (FR-014). Turning the integration off and choosing GitHub do not touch it.

## 5. What is never done

- The user query is not written to a log line (`tracing`), a notification, a toast, a session's
  history or `settings.json` (FR-016); `Debug` of `JiraQuery` is not logged.
- No request runs while the Jira source is not showing, the integration is off or the project is
  not connected (533 FR-020).
