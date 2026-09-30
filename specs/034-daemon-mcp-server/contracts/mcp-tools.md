# Contract: the tools

**Feature**: 034-daemon-mcp-server | Spec: *Operations*, FR-008–FR-018

All tools are scoped to the calling project (FR-010). `session` is a session UUID from
`list_sessions`; `worktree` is a directory name from `list_worktrees`, or `default`.

## Results

- **Success**: `{"content":[{"type":"text","text":<JSON of the output, pretty>}],
  "structuredContent":<output object>, "isError":false}`.
- **Failure** (FR-013): `{"content":[{"type":"text","text":"<category>: <message>"}],
  "structuredContent":{"error":{"category":<category>,"message":<message>}}, "isError":true}`,
  where `category ∈ not_found | invalid_input | conflict | refused_by_policy | needs_confirmation |
  service_error`. A failure changes nothing (FR-013).
- Unknown tool name or arguments that do not match the schema: failure with `invalid_input`.

## Rows

`WorktreeRow` = `{ref, display_name, branch|null, path, status: clean|missing|locked|prunable,
app_created: bool, assistant_owned: bool, session_count}`.
`SessionRow` = `{ref, label, ai_cli: "claude_code"|"copilot"|"pi"|"regular_terminal", lifecycle:
idle|starting|running|restarting|failed|interrupted_resumable, activity: unknown|working|
awaiting_input|ended, worktree: <ref>, is_caller: bool}`.

## Tools

| Tool | Input | Output | Policy (evaluated before anything changes) |
|---|---|---|---|
| `whoami` | — | `{session, project:{name,path}, worktree, ai_cli}` | — |
| `list_worktrees` | `{include_hidden?: bool=false}` | `{worktrees:[WorktreeRow]}` — `default` first, then the sidebar's set; hidden (feature 014/029 assistant-owned) rows only with `include_hidden` | — |
| `list_branches` | — | `{branches:[{name, kind: local|remote, checked_out_in: <ref>|null, unavailable_reason: string|null}]}` — the reason is the dialog pre-flight's `BranchSituation` wording (`null` when a new worktree can use it) | — |
| `list_sessions` | `{worktree?: ref}` | `{sessions:[SessionRow]}` | unknown `worktree` → not_found |
| `get_session` | `{session}` | `SessionRow + {failure_reason?}` | — |
| `read_session_output` | `{session, lines?: int=200}` | `{lines:[string], truncated: bool}` | self → invalid_input (FR-015); `lines<1` → invalid_input; `>2000` clamped (FR-012); FR-016 Off → refused_by_policy |
| `create_worktree` | `{branch, name?, mode?: new_branch|existing_local|track_remote = new_branch, remote?}` | `WorktreeRow` | Default caller → refused_by_policy (FR-015a); invalid name → invalid_input with the dialog's message; pre-flight mismatch → conflict naming the situation |
| `rename_worktree` | `{worktree, display_name}` | `WorktreeRow` | Default caller → refused_by_policy; `default` → invalid_input |
| `delete_worktree` | `{worktree, stop_sessions?: bool=false, delete_branch?: bool=true}` | `{removed: ref, branch_deleted: bool, leftovers:[path]}` | Default caller → refused_by_policy; caller's own worktree → refused_by_policy (FR-015); `default` → invalid_input; live sessions and `!stop_sessions` → conflict naming them; then **confirm** (FR-014) |
| `create_session` | `{worktree, ai_cli?: claude_code|copilot|pi, prompt?: string}` | `{session, lifecycle, prompt_delivered: bool|null, prompt_reason?: string}` | CLI availability checked **before** the record is created: not installed → service_error naming it, no record (US2 s5); `null` when no prompt was given |
| `start_session` | `{session}` | `{lifecycle}` | already Starting/Running/Restarting → success, unchanged (FR-012a) |
| `stop_session` | `{session}` | `{lifecycle}` | self → refused_by_policy (FR-015); Idle → success, unchanged; other session → **confirm**; effect: processes end, record `idle`, catalog broadcast |
| `interrupt_session` | `{session}` | `{}` | self → invalid_input (FR-015); not running → conflict (FR-012a); other session → **confirm** |
| `send_session_input` | `{session, text}` | `{}` | self → invalid_input; empty → invalid_input (FR-012a); FR-016: Auto → proceed, Confirm each send → **confirm**, Off → refused_by_policy |
| `delete_session` | `{session}` | `{}` | self → refused_by_policy (FR-015); then **confirm** |

Order of checks: scope (not_found) → input validation (invalid_input) → policy (refused_by_policy)
→ state conflicts (conflict) and no-ops → confirmation → effect. So a refused, conflicting or no-op
request never shows a prompt (FR-014).

**confirm** = the FR-014 flow ([protocol-delta.md](./protocol-delta.md) §2): allowed → proceed;
declined → `refused_by_policy` "declined by the user"; 60 s or no window → `needs_confirmation`;
target or caller gone → `not_found`.

`create_session` with `prompt`: returns once the prompt is written when the session first reports
ready for input (research R12; "output settled" = the primary terminal produced output and then
none for 1.5 s), or with `prompt_delivered: false` 60 s after the request or on a
failed start (FR-017). When the CLI's own trust record shows it would first ask whether to trust
the session's folder, nothing is typed and the call returns at once with `prompt_delivered: false`
(R12). `prompt_reason` is present exactly when `prompt_delivered` is `false`, and says which of the
three happened.

If the agent's HTTP connection closes while a request waits for confirmation, the request is
abandoned: the prompt is withdrawn and nothing changes.

## Logging (FR-018)

Each mutating call writes one `info` line: `caller=<uuid> op=<tool> target=<ref> outcome=<ok|category>`.
`prompt` and `text` never appear in any log line, at any level.
