# Contract: setting and create_worktree text

- `DaemonSettings.worktree_guard_enabled: bool` (default `true`), set through the existing settings edit request as `Option<bool>`; protocol version bumped. One app-wide value; applies to sessions launched after the change.
- UI: a toggle row in Settings beside "Let AI sessions manage worktrees and sessions", label "Guard sessions to their own worktree".
- `create_worktree` result: the existing JSON plus a `next_step` string field and the same sentence appended to the text content: "Do not work in this worktree from this session. Hand the work to a dedicated session with create_session for this worktree." (FR-010).
