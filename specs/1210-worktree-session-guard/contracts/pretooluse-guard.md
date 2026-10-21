# Contract: PreToolUse guard response

Request: existing `POST /hook/<session-uuid>` with bearer token; body has `hook_event_name: "PreToolUse"`, `tool_name`, `tool_input`, `cwd`.

Deny (HTTP 200): `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"Blocked: <tool> targets <path>, which is in another worktree. Do not work here; hand the work to a dedicated session with create_session."}}`

Allow: unchanged empty 200. Fail open (FR-009): empty 200 plus a warn log naming the reason. Tools judged: `Edit`, `Write`, `MultiEdit`, `NotebookEdit` (`file_path`/`notebook_path`), `Bash` (`command`, `cwd`). Reads are never judged. Body is still never logged (hooks.md rule 5): only the verdict and reason are.

Static deny rules added to `--settings`: `permissions.deny = ["EnterWorktree","Bash(git worktree add:*)","Bash(git * worktree add*)"]`; `mcp__micold__create_worktree` is not listed. Nothing else in the file changes shape except `permissions`.
