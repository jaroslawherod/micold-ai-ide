# Contract: Jira filter source (core `micold_core::jira_filter`, client `JiraCloud`)

Extends feature 533's [jira-cloud-source.md](../../533-jira-issue-worktrees/contracts/jira-cloud-source.md).
Nothing here changes `list_jql`, `search_jql`, `load`, `list_open` or `search_open`.

## 1. Clauses (pure)

`JiraFilters::clauses()` returns, in this order, one entry per chosen filter:

| Filter | Clause |
|---|---|
| reporter | `reporter = <q(account id)>` |
| label | `labels = <q(label)>` |
| component | `component = <q(name)>` |
| fix version | `fixVersion = <q(name)>` |

`q(v)` (`jql_string`) = `"` + `v` with `\` → `\\`, then `"` → `\"` + `"`. The value is never
trimmed, cut or case-changed. `search_jql` is not edited: it keeps its own identical escape line
(deliberate overlap, research R2), and `crates/micold-core/tests/jira_jql.rs` stays green unchanged.

`narrowed(jql, filters)`: with no filter chosen, `jql` unchanged; otherwise `jql` with
` AND <clause>` for each clause inserted before its last ` ORDER BY `. `jql` is always the output
of `list_jql` or `search_jql` (both end in ` ORDER BY updated DESC`); a `jql` without
` ORDER BY ` gets the clauses appended.

Examples (project `ABC`):
- component `Payments`, fix version `2.4`:
  `project = "ABC" AND statusCategory != Done AND component = "Payments" AND fixVersion = "2.4" ORDER BY updated DESC`
- label `a" OR project = "XYZ\`: `... AND labels = "a\" OR project = \"XYZ\\" ORDER BY updated DESC`

## 2. Trait additions (`JiraSource`)

```rust
fn list_filtered(&self, conn, token, filters: &JiraFilters, cursor: Option<&str>)
    -> Result<IssuePage, IssueLoadError>;            // default: list_open if filters empty, else Err
fn search_filtered(&self, conn, token, text: &str, filters: &JiraFilters)
    -> Result<Vec<Issue>, IssueLoadError>;           // default: search_open if empty, else Err
fn components(&self, conn, token) -> Result<Vec<String>, IssueLoadError>;        // default Ok([])
fn versions(&self, conn, token) -> Result<Vec<VersionChoice>, IssueLoadError>;   // default Ok([])
fn myself(&self, conn, token) -> Result<Option<JiraUser>, IssueLoadError>;       // default Ok(None)
fn project_users(&self, conn, token, text: &str) -> Result<Vec<JiraUser>, IssueLoadError>; // default Ok([])
```

`load_filtered(source, conn, token, filters) -> Result<IssueListing, IssueLoadError>`: `load`'s
paging and 1,000 cap over `list_filtered`.

`FakeJiraSource`: `with_components`, `with_versions`, `with_myself`, `with_users` script answers;
`list_filtered`/`search_filtered` record `(filters.clauses(), cursor | text)` and answer from the
existing page/search scripts; `filtered_calls()`, `choice_calls()` expose what was asked.

## 3. Requests (`JiraCloud`, `crates/micold-client/src/shell/jira_http.rs`)

| Method | Request | Sends |
|---|---|---|
| `list_filtered` | `POST /rest/api/3/search/jql`, `jql = narrowed(list_jql(key), f)` | project key, filter values, cursor |
| `search_filtered` | same, `jql = narrowed(search_jql(key, text)?, f)`, 50 results | + search text |
| `components` | `GET /rest/api/3/project/{key}/components` | project key |
| `versions` | `GET /rest/api/3/project/{key}/versions` | project key |
| `myself` | `GET /rest/api/3/myself` | nothing but credentials |
| `project_users` | `GET /rest/api/3/user/viewissue/search?projectKey={key}&query={text}&maxResults=50` | project key, typed text |

All: the connection's site only, headers `Authorization`, `Accept`, `User-Agent` (and
`Content-Type` on POST), 10 s budget, no redirect, failures through `classify` (533 §6). One
private `request(method, path, query, body)` builds every request so headers and the outcome
mapping exist once (the status bisection keeps using it). `project_users` with empty or
whitespace-only text answers `Ok([])` without a request.

## 4. Parsing (pure, core)

| Function | Body | Result |
|---|---|---|
| `parse_components` | `[{"name": …}, …]` | names, Jira order |
| `parse_versions` | `[{"name", "archived", "released"}, …]` | unarchived; unreleased (Jira order) then released (reversed) |
| `parse_myself` | `{"accountId", "displayName"}` | `Some(JiraUser { is_me: true })`, `None` if unusable |
| `parse_users` | `[{"accountId", "displayName"}, …]` | users, Jira order |

Every parser: a body that is not JSON of that shape → `IssueLoadError::NotJiraCloud`; an entry
whose value (name, account id, display name) is empty or holds a control character is skipped;
duplicates by value (by account for users) are dropped, first kept.

`label_choices(&IssueListing) -> LabelChoices` — see [data-model §4](../data-model.md).
