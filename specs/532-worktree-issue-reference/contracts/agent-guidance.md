# Contract: agent-facing guidance strings

Wording is the implementer's; each string MUST contain the listed content, and tests assert it.

## C1. `create_worktree` description (`mcp/tools.rs`)

MUST: name `${type}/${ticket}_${name}` (branch) and `${type}-${ticket}_${name}` (directory); say
to include the issue reference whenever the worktree is for an issue or ticket; say to pass it as
`ticket` (with `type` and `name`) or `github_issue`, not inside a literal `branch`; keep the
literal-branch alternative, modes, collision and result text.

## C2. `ticket` property description

MUST: keep "slugified as the form does; blank means none"; add that the ticket is what makes the
sidebar issue tag and the issue filter work, and that a worktree without it gets neither.

## C3. `INSTRUCTIONS` (`mcp/jsonrpc.rs`)

MUST: keep the scope and confirmation sentences; add one sentence: when a worktree is for an issue
or ticket, include the issue reference in its name, by passing `ticket` (or `github_issue`) to
`create_worktree`, which names it `${type}/${ticket}_${name}`.

## Unchanged

Tool inputs, input validation, result shape and refusal texts. A call with only `branch` returns
what it returned before.
