# Contract: the Jira Cloud issue source

**Feature**: [spec.md](../spec.md) | Research [R1](../research.md#r1--reuse-the-034038-pipeline-with-a-second-provider), [R2](../research.md#r2--http-client-ureq-3-in-the-client-not-curl-not-reqwest), [R6](../research.md#r6--jira-cloud-requests-enhanced-jql-search-one-endpoint), [R7](../research.md#r7--failure-classification)

## 1. The trait (`micold_core::jira`)

```rust
pub trait JiraSource {
    /// One page of open issues, most recently updated first; `cursor` is Jira's nextPageToken.
    fn list_open(&self, conn: &JiraConnection, token: &ApiToken, cursor: Option<&str>)
        -> Result<IssuePage, IssueLoadError>;
    /// Open issues matching `text` on Jira's side; the caller applies FR-012's rule.
    fn search_open(&self, conn: &JiraConnection, token: &ApiToken, text: &str)
        -> Result<Vec<Issue>, IssueLoadError>;
    /// Jira's count of open issues, or None when it cannot be read (never an error).
    fn count_open(&self, conn: &JiraConnection, token: &ApiToken) -> Option<u64>;
}
```

`JiraCloud` (`micold-client/src/shell/jira_http.rs`) implements it with `ureq`;
`FakeJiraSource` (core) scripts pages and failures. `load_listing` and the `complete`/cap rule are
the shared helper of [R1](../research.md#r1--reuse-the-034038-pipeline-with-a-second-provider):
`ISSUE_LOAD_CAP` 1,000, truncating the crossing page, `complete` when no cursor remains.

## 2. Requests

| Call | Method and path | Body |
|---|---|---|
| list | `POST {origin}/rest/api/3/search/jql` | `{"jql":"project = \"KEY\" AND statusCategory != Done ORDER BY updated DESC","fields":["summary","issuetype","labels","reporter","description","updated"],"maxResults":100,"nextPageToken":<cursor>}` |
| search | same | JQL of §3 |
| count | `POST {origin}/rest/api/3/search/approximate-count` | `{"jql":"project = \"KEY\" AND statusCategory != Done"}` |

Headers: `Authorization: Basic base64(email ":" token)`, `Accept: application/json`,
`Content-Type: application/json`, `User-Agent: micold-ai-ide`. Nothing else is sent (FR-020): no
cookies, no telemetry, no other project. Redirects are not followed (a redirect is
`NotJiraCloud`). Timeout 10 s total per request (FR-018).

## 3. JQL building (pure, `jira::jql`)

- `list_jql(key) -> String` as above; `key` is a validated `ProjectKey`, so it cannot carry a quote.
- `search_jql(key, text)`: `... AND (summary ~ "<t>*" OR labels = "<t>" [OR key = "<KEY-n>"])`.
  `text` is trimmed; `\` and `"` are escaped; control characters and newlines are removed; at most
  100 characters; the `key = ` clause is added only when `text` matches `^[A-Za-z][A-Za-z0-9_]+-\d+$`
  after upper-casing and its prefix equals the project key. An empty text builds no search.

## 4. Parsing (pure, `jira::parse_page`)

`{"issues":[{"key","fields":{"summary","issuetype":{"name"},"labels":[...],
"reporter":{"displayName"}|null,"description":<ADF>|null,"updated"}}],"nextPageToken","isLast"}`.

- Key `ABC-123` -> `Issue::jira`; a key without `-<digits>` skips the node.
- `reporter.displayName`, else `emailAddress`'s local part, else the ghost login; never the
  account id.
- `description`: ADF flattened by walking `content` arrays, taking `text` nodes, `hardBreak` and
  paragraph boundaries as a space; passed to `Issue::described` (`description_from`, 600 chars).
- At most 20 labels kept, as GitHub (R2 of 034).
- A body that is not JSON, or lacks `issues`, is `NotJiraCloud`.

## 5. Search

Same rule as 034 FR-005a: only when the listing is `!complete` and the query is non-empty, after
the 300 ms debounce the shell runs `search_open`; results merge through `merge_searched`
(de-duplicated by `number`) and are kept only if they match the query by the local rule.

## 6. Classification and notices (`jira::classify`, `IssueLoadError::message`)

| Outcome | Variant | Notice text |
|---|---|---|
| connect, DNS, TLS, proxy failure | `Offline` | "Couldn't reach {host}: {reason}." (reason <= 80 chars, never the token) |
| no answer in 10 s | `TimedOut` | "{host} did not answer within 10 seconds." |
| 401; 403 | `CredentialsRefused` | "{host} refused the credentials. Check the email and API token in Settings > Jira." |
| 400 or 404 for the project | `NoAccess` | "{host} does not show project {KEY} to this account. Check the project key and your access in Settings > Jira." |
| 429 | `RateLimited` | "{host} is limiting requests. Try again in a minute." |
| not Cloud v3 | `NotJiraCloud` | "{host} did not answer like Jira Cloud. Jira Data Center and Server are not supported." |
| anything else | `Other` | "Jira returned an error ({status})." |

Every notice has **Retry** (FR-018, US4 AS5-7); none prints the token, the email or the response body.
Fixtures for each outcome live in `crates/micold-core/tests/fixtures/jira/`.
