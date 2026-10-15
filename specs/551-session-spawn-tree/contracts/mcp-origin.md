# Contract: MCP rows

`list_sessions` and `get_session` rows gain, when the session has an origin:

- `origin`: the origin's `ref` (same form as `ref`);
- `origin_state`: `"known"` or `"removed"`.

Both are omitted for a top-level session. The origin is what the session's provider reported
(FR-001); `create_session` records none (FR-019) and its result row carries neither field. The
`ref` is emitted from the stored id whatever the status; `get_session` on the ref of a `removed`
origin returns not_found (an archived session no longer resolves).

No input parameter is added: an agent cannot name an arbitrary origin.
