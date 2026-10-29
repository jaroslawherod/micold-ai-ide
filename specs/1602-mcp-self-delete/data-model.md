# Data model: MCP self stop and own-worktree delete (#1602)

No persisted data. All types are runtime or wire.

## `LossSummary` (wire, `micold-core/src/protocol/messages.rs`; built in `worktree/loss.rs`)

| Field | Type | Rule |
|---|---|---|
| `files` | `Vec<String>` | Changed and untracked paths, repository-relative, at most 50, in git's order. |
| `files_more` | `u32` | Count beyond the 50 shown; 0 when all are shown. |
| `commits` | `Vec<LossCommit>` | Unpushed commits, newest first, at most 50. |
| `commits_more` | `u32` | Count beyond the 50 shown. |
| `unreadable` | `bool` | Git failed or the directory is not a worktree: lists are empty, the dialog says so. |

`LossCommit { short_hash: String, subject: String }`.

Empty means `files`, `commits` empty and `unreadable` false: the dialog shows nothing (FR-006).
`WorktreeLoss` (the `Git` trait's return) carries the same data unbounded; `LossSummary::bounded` applies the 50.

## `ConfirmRequest` / `ConfirmationRequested` additions

| Field | Type | Rule |
|---|---|---|
| `loss` | `Option<LossSummary>` | `Some` only for `DeleteWorktree`. |
| `self_targeted` | `bool` | From `is_self_targeted`; the dialog says the asking session ends. |

## `Deferred` (daemon, `mcp/tools.rs`; not serialized)

```text
Deferred::DeleteWorktree { project, dir_name, delete_branch, label }   // runs ops::delete_worktree, stop_sessions = true
Deferred::StopSession    { project, session, label }
Deferred::DeleteSession  { project, session, label }
```

Built only after `ask_user` returned `Ok`. State transitions: `built` to `reply written` to `peer closed or
2 s elapsed` to `run`; `run` ends `done` or `failed` (log plus `DeferredActionFailed`). No retry, no
rollback (spec Out of Scope).

## `DaemonMsg::DeferredActionFailed`

`{ project: PathBuf, subject: String, reason: String }`. Sent to every window, never answered.

## `PolicyDecision`

Unchanged shape. `decide` returns `Confirm(op)` for self-targets; `is_self_targeted` is a separate pure fn so
the existing `ConfirmedOp` wire form is untouched.
