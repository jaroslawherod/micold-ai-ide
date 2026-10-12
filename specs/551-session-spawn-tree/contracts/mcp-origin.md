# Contract: MCP rows

`list_sessions` and `get_session` rows gain, when the session has an origin:

- `origin`: the origin's `ref` (same form as `ref`);
- `origin_state`: `"known"` or `"removed"`.

Both are omitted for a top-level session. `create_session` records the calling session as the
origin of the created session when `worktree` is the caller's own worktree (FR-019); the result row
shows it. The `ref` is emitted from the stored id whatever the status; `get_session` on the ref of a
`removed` origin returns not_found (an archived session no longer resolves). `create_session`'s
result row carries the same fields.

No input parameter is added: an agent cannot name an arbitrary origin.
