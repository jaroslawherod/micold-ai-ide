# Contract: reading one issue's details

## 1. Trait

```rust
pub trait IssueDetailsSource: Send + Sync {
    fn read_details(&self, repo: &GithubRepo, number: u32, deadline: std::time::Instant)
        -> Result<IssueDetails, IssueLoadError>;
}
pub trait IssueLookup: IssueDetailsSource { /* read_issue as in 550 */ }
pub trait IssueSource: IssueDetailsSource { /* list_open, search_open, describe_open as in 034/038 */ }
```

- **`GhCli`**: runs the main query, then timeline pages (§4), each run bounded by
  `deadline - now`. No run starts after the deadline. No retry.
- **`FakeIssueSource`**:
  - `with_details(Result<IssueDetails, IssueLoadError>)` scripts the answer, and
    `with_details_delay(Duration)` delays it.
  - `details_calls() -> Vec<(String, u32)>` records each call.
  - With nothing scripted, it answers `Err(IssueLoadError::Other("no details scripted"))`, so a test
    that does not care still runs.

## 2. Request

`gh api graphql --hostname github.com -f query=<DETAILS_QUERY> -f owner=<o> -f name=<r> -F n=<number>`.
The same argument shape as `lookup_args` (550): only owner, name and number leave the machine
(FR-009). It runs from the user's home with `GH_PROMPT_DISABLED=1` like every `GhCli` run.

`DETAILS_QUERY` fields: research R3. `RELATED` stands for
`number title state repository { nameWithOwner }`, and `PR` for `RELATED isDraft merged`.

```graphql
query($owner: String!, $name: String!, $n: Int!) { repository(owner: $owner, name: $name) {
  issue(number: $n) {
    number
    issueDependenciesSummary { blockedBy totalBlockedBy blocking totalBlocking }
    blockedBy(first: 20) { totalCount nodes { RELATED } }
    blocking(first: 20) { totalCount nodes { RELATED } }
    parent { RELATED }
    subIssuesSummary { total completed }
    subIssues(first: 20) { totalCount nodes { RELATED } }
    labels(first: 20) { totalCount nodes { name } }
    assignees(first: 20) { totalCount nodes { login } }
    milestone { title }
    projectItems(first: 20) { totalCount nodes { project { title }
      fieldValues(first: 20) { nodes { __typename <value fragments with field name> } } } }
    closedByPullRequestsReferences(first: 20, includeClosedPrs: true) { totalCount nodes { PR } }
    timelineItems(itemTypes: [CROSS_REFERENCED_EVENT], first: 100) {
      pageInfo { hasNextPage endCursor } nodes { ... on CrossReferencedEvent { source { ... on PullRequest { PR } } } } }
} } }
```

The query is kept on one line in code, as `ISSUE_QUERY` is. Field names are confirmed against a live
recording in the first core task (research R3).

## 3. Parse: `parse_details(stdout, repo) -> Result<IssueDetails, IssueLoadError>`

| Answer | Result |
|---|---|
| not JSON | `Err(Other("GitHub's answer could not be read: …"))` |
| no `data` object | `Err(graphql_error(..))`: the existing typing (RATE_LIMITED, FORBIDDEN, …) |
| `data.repository` null | `Err(NoAccess)` |
| `repository.issue` null | `Err(Other("GitHub's answer had no issue"))` (550 already refused it) |
| `issue` present | `Ok(details)`: each `errors[]` entry with `path = ["repository","issue",<field>,…]` marks the kind owning `<field>` as `NotReadable(reason)` |

Field to kind: `blockedBy` and `issueDependenciesSummary.blockedBy*` go to `blocked_by`;
`blocking`/`…blocking*` to `blocking`; `parent` to `parent`; `subIssues`/`subIssuesSummary` to
`sub_issues`; `labels`, `assignees`, `milestone` to themselves; `projectItems` to `projects`;
`closedByPullRequestsReferences` to `closing_prs`; `timelineItems` to `linked_prs`.

The reason is one plain clause from the error's `type`:

| `type` | Reason |
|---|---|
| `INSUFFICIENT_SCOPES` | "your GitHub sign-in can't read projects (it lacks the read:project scope)", with the kind named in place of "projects" |
| `FORBIDDEN` | "GitHub refused access" |
| anything else | "GitHub could not provide it" |

`gh` exits non-zero on partial errors. As with `GhCli::run` today, an answer the parser accepts is
used whatever the exit status.

## 4. Timeline pages (linked PRs, research R4)

- **When**: while `timelineItems.pageInfo.hasNextPage`, fewer than 10 pages have been read, and the
  deadline has not passed.
- **What**: run `TIMELINE_QUERY` (the same variables plus `-f after=<endCursor>`; only the
  `timelineItems` connection) and add its PR sources.
- **Stop**: when the loop ends with `hasNextPage` still true, set `total_is_lower_bound`. A page
  that fails also sets it and keeps what was read. A failed page never fails the whole read.

## 5. Normalising

- **Linked PRs**: drop repeats (by reference) and drop every PR that is in the closing list.
  `total` is the number left.
- **Hidden** (research R5): `summary_total - connection.totalCount` (floor 0), plus `null` nodes.
- **Empty values**: empty field values and empty-after-neutralising strings are left out (data-model §1).
- **Order**: GitHub's order is kept throughout.
