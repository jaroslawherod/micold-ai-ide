# Contract: tool-surface delta (amends 034 `contracts/mcp-tools.md`, FR-014a)

## `delete_worktree`

- `worktree` may name the caller's own worktree. No policy refusal. `"default"` stays invalid input;
  a Default caller stays refused by policy (FR-007).
- Own worktree requires `stop_sessions: true`, else `conflict` "worktree ... has live sessions" (FR-002).
- Order of checks: input, scope (`not_found`), policy, live-session conflict, confirmation, then act.
- Prompt: names caller, operation, worktree; lists uncommitted files and unpushed commits (FR-006).
- Result when the target is the caller's own worktree (FR-008), returned before anything stops:
  `{ "scheduled": true, "removed": "<dir_name>", "branch_will_be_deleted": <bool> }`.
  The text says the stop and removal are scheduled, not done. Other targets keep
  `{ removed, branch_deleted, leftovers }` (FR-013).

## `stop_session`, `delete_session`

- `session` is optional. Omitted or equal to the caller's id: self-targeted, confirmed, result
  `{ "scheduled": true }` (stop; no `lifecycle`, it has not stopped yet) or `{ "scheduled": true }` (delete), then the action.
- Another session: unchanged.
- `interrupt_session`, `send_session_input`, `read_session_output` on self: `invalid_input`, unchanged.

## Errors for a self-targeted call

Decline `refused_by_policy` ("declined by the user"); no window or 60 s `needs_confirmation`; withdrawn
prompt `not_found`. No state changes in each case (FR-005).

## Audit line (FR-010)

One `micold::mcp` line per mutating call with `outcome` `ok` or the category, written when the result is
produced, so before the deferred step. A failed deferred step adds its own `error` line (R5).

## `tools/list` text (FR-012)

Remove: "A session cannot stop or delete itself", "\"default\" and the calling session's own worktree cannot
be deleted". Add: the confirmation, the scheduled result, and that the session ends after its answer.
