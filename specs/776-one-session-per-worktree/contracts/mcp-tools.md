# Contract: MCP tools

Descriptions in `micold-core/src/mcp/tools.rs` and `docs/user-guide/agent-tools.md` change with
the behaviour (FR-014). Schemas change only where stated.

## `create_session`

Input unchanged. New failure, checked after scope and worktree resolution and policy, before the
CLI availability check, and enforced again under the catalog lock (R2):

- Category `Conflict`. Message:
  `this worktree already has a session: "<name>" (<id>); start_session on <id> brings it back`
  (Default: `the project root already has a session: ...`).
- The catalog is unchanged; no process starts.
- Two simultaneous calls on an empty worktree: one succeeds, the other gets this error naming the
  winner.

Description gains: "A worktree has at most one session: a second is refused, naming the
existing one."

## `start_session`

Input unchanged. Still a no-op reporting success for a starting, running or restarting session
(FR-009) and a start of the same session when stopped. New failure (legacy catalogs only): category
`Conflict`, `session "<name>" (<id>) is running in this worktree; stop it first`.

## `create_worktree`

Adds optional inputs, valid with both the literal (`branch`, `name`) and derived forms:

| Input | Type | Meaning |
|---|---|---|
| `ai_cli` | enum of `AiCli::ALL.map(tool_name)` | the new worktree's session CLI; unnamed with `prompt` given means the user's default |
| `prompt` | string | the session's first input, typed once the CLI is ready |

- Neither given: the 550 call, byte-for-byte the same result (no session).
- Either given: the session is created in the new worktree. Policy is checked as
  `CreateWorktree` and as `CreateSession` for the new worktree. A missing CLI fails with
  `cli_reason`'s text before any worktree exists.
- Reply: the existing worktree row plus `session` (id), `lifecycle`, and, when `prompt` was
  given, `prompt_delivered` and `prompt_reason` exactly as `create_session` returns them.
- Worktree created but session not: error naming the worktree; it has no session and
  `create_session` on it works (FR-010).

## Unchanged

`delete_session` (archive; frees the location, FR-010), `attach_worktree`, `list_sessions`,
`list_resumable_sessions` (no tool resumes a provider session, so FR-008b has no MCP surface).
