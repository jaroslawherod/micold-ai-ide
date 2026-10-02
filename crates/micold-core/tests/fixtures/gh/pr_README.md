`gh api graphql --hostname github.com --include -f query="$QUERY" -f owner=<owner> -f name=<name> -f b0=<branch 0> … > pr_<name>.txt`

# The recorded pull request answers (`pr_*.txt`)

What `gh api --include` wrote to stdout for one pull request request of feature 040
(`specs/040-worktree-pr-ci-status/contracts/pull-request-source.md` §2 and §5): the HTTP status
line, the headers, an empty line, the body. Recorded on 2026-10-02 with `gh` 2.54.0 on Linux, with
the command of the first line. `$QUERY` is the document `pull_request::status_query(n)` returns for
the `n` branches passed; `tests/pull_request_query.rs` pins it.

After recording, three header lines were removed and nothing else was changed:
`X-Github-Request-Id`, `X-Oauth-Scopes` and `X-Oauth-Client-Id`. `gh` prints no token and no
`Authorization` header; `tests/pull_request_parse.rs` holds that for every file.

The files keep the `\r\n` line ends `gh` prints after the status line and each header.
`.gitattributes` marks them `-text` so no checkout rewrites them.

The pull requests of other repositories are public ones, read without changing anything. Their
state on GitHub has moved on since; the files are what GitHub answered on the day.

| File | Repository | Branches asked for (`b0`, `b1`, …) | What the answer holds |
|---|---|---|---|
| `pr_three_branches.txt` | `jaroslawherod/micold-ai-ide` | `release-please--branches--main--components--micold-ai-ide`, `feat/worktree-pr-ci-status`, `no-such-branch-040` | #347 open (beside older merged and closed ones of the same branch); #536 merged (newer than #529, merged); nothing for the third |
| `pr_no_checks.txt` | `jaroslawherod/micold-ai-ide` | `release-please--branches--main--components--micold-ai-ide` | #347 open, `statusCheckRollup: null`, `reviewDecision: null` |
| `pr_closed.txt` | `jaroslawherod/micold-ai-ide` | `fix/macos-packaging-doc-spec-links` | #302 closed without merging, no open one |
| `pr_draft.txt` | `cli/cli` | `williammartin-extension-browse-bubbletea` | #14578 open, draft |
| `pr_checks_passing.txt` | `cli/cli` | `bagtoad/artifact-list` | #14566 open: 12 `SUCCESS`, 13 `SKIPPED` |
| `pr_checks_failing.txt` | `microsoft/vscode` | `dependabot/github_actions/github/codeql-action/init-4.38.2` | #339083 open: 2 `FAILURE` beside 32 `SUCCESS` and 1 `NEUTRAL` |
| `pr_checks_pending.txt` | `microsoft/vscode` | `agents/swap-branch-new-worktree-order` | #339346 open: 4 `IN_PROGRESS` beside 31 `SUCCESS`, none failed |
| `pr_review_states.txt` | `microsoft/vscode` | `agents/model-picker-auto-toggle-centering`, `aashna-hydrafusion-agent-host-cost-parity`, `agents/swap-branch-new-worktree-order` | #339339 `APPROVED`, #339165 `CHANGES_REQUESTED`, #339346 `REVIEW_REQUIRED` |
| `pr_cross_repository.txt` | `cli/cli` | `patch-1` | ten pull requests whose head branch is a fork's `patch-1` (`isCrossRepository: true`), one of them open (#14373); none lives in `cli/cli` |
| `pr_repo_not_found.txt` | `jaroslawherod/no-such-repository-040` | `main` | HTTP 200 with a `NOT_FOUND` error at `["repository"]`; `gh` exited 1 and wrote `gh: Could not resolve to a Repository with the name 'jaroslawherod/no-such-repository-040'.` to stderr |
| `pr_truncated.txt` | — | — | the first 2,000 bytes of `pr_three_branches.txt` (`head -c 2000`): an answer cut mid-body |

## Written from GitHub's documented answer

A request limit cannot be produced on demand without spending the sign-in's whole budget, so these
three are not recordings. Each is the header block of `pr_no_checks.txt` with the status line and
the rate-limit headers replaced, and the body GitHub documents
(<https://docs.github.com/en/graphql/overview/rate-limits-and-query-limits-for-the-graphql-api>).

| File | What it is |
|---|---|
| `pr_rate_limited_graphql.txt` | the primary limit: HTTP 200, a `RATE_LIMITED` error, `X-Ratelimit-Remaining: 0` and `X-Ratelimit-Reset: 1790966100` |
| `pr_rate_limited_secondary.txt` | a secondary limit: HTTP 403, `Retry-After: 30`, `X-Ratelimit-Remaining: 4990` |
| `pr_rate_limited_secondary_no_retry_after.txt` | the same answer without `Retry-After` |

The failures `gh` reports only on stderr (not signed in, offline, SAML, a missing scope) are feature
034's `*.stderr` files in this directory, read through `github::classify`; they are not copied.
