# Contract: MCP rows and arguments

Same style as `specs/551-session-spawn-tree/contracts/mcp-origin.md`.

- `create_worktree`: new optional input `parent` (a worktree ref: `default` or a worktree's directory name). Omitted: the caller's worktree (`default` for a root session; none if it cannot be resolved). Not a live worktree of the project: `invalid_input`, nothing created. Result row gains `parent` and `parent_state`.
- `list_worktrees`: rows gain `parent` (ref form; for a removed parent its last name) and `parent_state` (`"known"` | `"removed"`); both omitted when unparented. New optional input `parent` filters to direct children; `default` allowed; an unknown name returns an empty list, not an error.
- `list_sessions`, `get_session`: rows gain the same two fields, taken from the session's worktree; omitted for a session in `default` or in an unparented worktree.
- `rename_worktree`, `delete_worktree`: no argument change. Delete of a parent leaves children listed with `parent_state: "removed"`.
- No tool sets or changes a parent after creation (FR-004).
- **Visibility (FR-015, FR-016)**: for a caller not in `default`, `list_worktrees`, `list_sessions`, `get_session`, `read_session_output`, `send_session_input`, `interrupt_session`, `start_session`, `stop_session`, `create_session`, `rename_worktree`, `delete_worktree` and `report_review_comment` act only on {own worktree, parent, siblings, direct children}; any other ref returns `not_found`, and listings omit it. `whoami` is unchanged. `create_worktree` accepts an explicit `parent` only from that set (else `invalid_input`). A caller in `default` is unrestricted. There is no relay tool: reaching a session outside the set is done by messaging the parent (visible to it), which forwards it with its own `send_session_input` (FR-017). The existing permission policy still applies on top.
