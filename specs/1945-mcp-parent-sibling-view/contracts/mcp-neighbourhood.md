# Contract: `neighbourhood` and `send_parent_input`

Style as `specs/1857-worktree-parent/contracts/mcp-parent.md`.

## `neighbourhood` (read-only, no input; extra properties rejected)

Result:

```json
{
  "worktree": { "ref": "A", "sessions": [ <session row, is_caller marked> ] },
  "parent": { "ref": "default", "state": "known", "recorded": false,
              "sessions": [ <rows> ] },
  "siblings": [ { "ref": "B", "sessions": [ <rows> ] } ],
  "children": [ { "ref": "C", "sessions": [ <rows> ] } ],
  "note": "no recorded parent: \"default\" is taken as the parent for visibility"
}
```

- Session rows are `list_sessions` rows (same fields, same `is_caller`).
- `parent` is `null` for a caller in `default`, with `note` "default has no parent". Its `children`
  include the unparented worktrees, each with `"implicit": true`.
- Removed parent: `parent` is `{ "ref": <last name>, "state": "removed", "recorded": true,
  "sessions": [] }`, `siblings` is `[]`.
- `recorded` is `false` only for the implicit `default`. `note` is omitted when the parent is recorded
  and known.
- Unknown worktree of the caller: `{ "worktree": null, "parent": null, "siblings": [], "children":
  [], "note": "this session's worktree is not known to the daemon" }`.
- Nothing outside the caller's scope appears (SC-005). Always allowed by policy, like `whoami`.

## `send_parent_input`

Input: `text` (non-empty string, as `send_session_input`). Result: the `send_session_input` result
plus `"session"` (the session reached) and `"parent_worktree"` (its ref).

Refusals (nothing is typed):

| Case | Category | Message names |
|---|---|---|
| Caller in `default` | `invalid_input` | "default has no parent" |
| Parent removed | `not_found` | the parent's last name, "was removed" |
| Parent has no live session | `conflict` | the parent worktree, "has no running session" |
| Caller's worktree unknown | `invalid_input` | "no parent could be determined" |

The parent target is the earliest-created running session of the parent worktree (R3). Everything
after the target is chosen is `send_session_input`: cross-session policy, confirmation, trust check,
atomic submission, "session gone". Logged like it: the text is never logged.

## Changed behaviour

Every tool that applies the 1857 FR-015 limit treats a worktree with no recorded parent as a child of
`default` (research R1). `list_worktrees` rows are unchanged: no `parent` for those worktrees.
