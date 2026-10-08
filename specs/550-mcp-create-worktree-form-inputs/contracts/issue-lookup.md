# Contract: single-issue lookup (extends 034-github-issue-worktree contracts/github-issue-source.md)

## Trait

```rust
pub trait IssueLookup: Send + Sync {
    fn read_issue(&self, repo: &GithubRepo, number: u32) -> Result<IssueSnapshot, IssueLookupError>;
}
```

`GhCli` implements it; one `gh` run bounded by `GH_TIMEOUT` (10 s), no retry.

## Request

`gh api graphql --hostname github.com -f query=<ISSUE_QUERY> -f owner=<o> -f name=<r> -F n=<number>`.
Only owner, name and number leave the machine (FR-025 of 034, FR-008 here). `n` is the only typed `-F`.

## Response classification

`gh` exits non-zero on a GraphQL error, so stdout is parsed first: `data.repository`/`repository.issue` decide, `errors[]` only when `data` does not. `parse_lookup` returns `Result<Result<IssueSnapshot, NotOpen>, IssueLoadError>`. A NOT_FOUND error alongside `issue: null` is `NotOpenIssue`, not `NoAccess`.

| Response | Result |
|---|---|
| `gh` missing / signed out / offline / rate limited / timeout / other | `Load(classify(..))`, message from `IssueLoadError::message(repo)` |
| `data.repository` null | `Load(NoAccess)` |
| `repository.issue` null | `NotOpenIssue { closed: false }` ("#N is not an open issue in o/r: it may be a pull request or not exist") |
| `state == "CLOSED"` | `NotOpenIssue { closed: true }` ("issue #N in o/r is closed") |
| `state == "OPEN"` | `IssueSnapshot { number, title, labels }` |
| unparseable | `Load(Other(..))` |

Not-open reasons are wrapped as `invalid_input`; load failures as the category the daemon already
uses for an unavailable dependency (`service_error`), per `OpError` conventions in `mcp/tools.rs`.
