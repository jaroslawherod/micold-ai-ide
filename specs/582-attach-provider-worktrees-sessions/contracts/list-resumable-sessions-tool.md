# Contract: `list_resumable_sessions` (MCP)

Read-only, allowed for every caller, never audited as a mutation. Touches no provider file (FR-010).

**Input**: `{ "limit"?: integer 1..=200 (default 50), "offset"?: integer >= 0 (default 0), "worktree"?: string }`.

**Result**: `{ "sessions": [ { "session": uuid, "provider": "claude_code"|"copilot"|"pi", "title": string|null, "last_activity": RFC3339, "worktree": ref|"default", "status": "resumable"|"needs_worktree_attach"|"unresumable", "reason": string|null } ], "total": integer (of the newest 200 the discovery returns), "notes": [ { "provider": string|null, "reason": string } ] }`, newest first.

A session already visible in the catalog is not listed (use `list_sessions`). The tool is informational for agents: resuming a listed session is the app's action (`AttachApply` then `SessionStart`). No MCP tool adopts a session; an agent that wants a session back calls `attach_worktree` for its worktree and finds the adopted sessions in `list_sessions` once it is attached (sessions recorded since the last open appear after the next open). Agent-side resume is out of scope (R8). Unreadable stores appear in `notes`, not as an error.
