# Contract: the worktree's branch set and its pull requests (#1003)

Replaces §1 of `specs/775-diff-view-session-prs/contracts/pull-request-reads.md`. §2 there (a pull
request's diff) is unchanged.

## 1. The branch set (local, read-only)

Run in the worktree's directory with the client's local `Git`, on the list read's blocking thread,
each bounded like the Changes view's other git runs:

```
git log -g --date=unix --format=%gD%x09%H%x09%gs HEAD --
git log -g --ignore-missing --date=unix --format=%gD%x09%H%x09%gs refs/heads/<c1> … refs/heads/<cn> --
```

Arguments are a `Vec`, no shell. Branch names come from git's own output, never from the user.
Nothing is written. The set is research R2's rules; it leaves the machine only as GraphQL variables
(below). No local git or a failed first run: the current branch alone. Default (project root): no
read.

## 2. Every pull request of the set

`gh api graphql --hostname github.com --include -f query=<list_query(n)> -f owner=<o> -f name=<r> -f b0=<branch> … -f b<n-1>=<branch>`

One request per 25 branches, one after another.

```graphql
query($owner: String!, $name: String!, $b0: String!, …) {
  repository(owner: $owner, name: $name) {
    defaultBranchRef { name }
    p0: pullRequests(headRefName: $b0, first: 100, orderBy: {field: CREATED_AT, direction: DESC}) { nodes { ...item } }
    …
  }
  rateLimit { remaining resetAt }
}
fragment item on PullRequest { number title url state isDraft createdAt isCrossRepository
  baseRefName headRefName headRefOid
  commits(last: 1) { nodes { commit { statusCheckRollup { contexts(first: 1) {
    checkRunCount checkRunCountsByState { state count }
    statusContextCount statusContextCountsByState { state count } } } } } } }
```

- No branch name is part of the document; `-f` keeps a branch named `true` or `@x` a string.
- Read: forks dropped; aliases of the default branch dropped; checks by `reduce_checks`; unknown
  `state` fails the read as `Passing` (as 775). Merged over chunks, sorted `createdAt` desc, then
  `number` desc.
- Failures classify with the existing `reading_failure` (`Unavailable`, `Passing`,
  `RateLimited{until}`); the first failing chunk fails the whole read.
- Sent: owner, repository name, branch names (as 040 FR-031). Nothing stored.
