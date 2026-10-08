# Quickstart: verifying create_worktree form inputs

## Part A: automated

```sh
mise run test-core                      # naming, github lookup parsing, tool parser and schema
cargo test -p micold-daemon --test mcp_create_worktree   # via mise run gate for the real run
mise run gate                           # before pushing
```

## Part B: by hand, real `gh` (signed in, a repo with a GitHub remote)

1. From an agent session, call `create_worktree` with `type: fix`, `ticket: "#123"`, `name: "login crash"`. Expect branch `fix/123_login-crash`, directory `fix-123_login-crash`, sidebar type `fix`, issue tag `123`, the result listing type, ticket, branch, directory.
2. Repeat the same call: refused naming the branch and advising `branch` + `mode`.
3. Call with `github_issue: <an open bug issue>`: ticket, name and type come from the issue and match what the New worktree form produces when that issue is picked.
4. Call with `github_issue` of a closed issue and of a pull request number: plain refusal, nothing created.
5. Sign out (`gh auth logout`) and call with `github_issue`: the form's "not signed in" text. Call without `github_issue`: works.
6. Call with `branch` and `type`: refused as alternatives.
7. Call as before with `branch` only: unchanged.
