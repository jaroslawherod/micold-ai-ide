# Contract: the two GitHub reads (#775)

Both run through `PullRequestSourceFactory` at the single call site in `shell/pr_status.rs` (the guard test `the_source_is_read_from_one_place` keeps holding; `shell/changes.rs` sends it a message and never touches `.pull_requests`) on a
blocking thread, bounded by `GH_TIMEOUT` (10 s), no retry, after `locate_gh` and the same remote
choice (`choose_remote`) as the status read. No GitHub remote or no `gh` is `Unavailable`.

## 1. List a branch's PRs
`gh api graphql --hostname github.com --include -f query=<list_query> -f owner=<o> -f name=<n> -f b=<branch>`

```graphql
query($owner: String!, $name: String!, $b: String!) {
  repository(owner: $owner, name: $name) {
    pullRequests(headRefName: $b, first: 100, orderBy: {field: CREATED_AT, direction: DESC}) {
      nodes { number title url state isDraft createdAt isCrossRepository baseRefName headRefOid }
    }
  }
  rateLimit { remaining resetAt }
}
```
No branch name is part of the document (variables only). Failures classify with the existing
`reading_failure` (`Unavailable`, `Passing`, `RateLimited`). Output order is GitHub's; the parser
sorts by `created_at` descending as a guard.

## 2. A PR's diff
`gh api --hostname github.com -H "Accept: application/vnd.github.v3.diff" repos/<o>/<n>/pulls/<N>`

Output: unified diff, `diff --git a/x b/y` sections. HTTP 406 (`too_large`; detected by a new `diff_failure(&RunOutcome)` reading the status line / stderr `HTTP 406`) maps to
`DiffReadFailure::TooLarge`; any other failure to `Failed`. Sent: owner, repo, number.
