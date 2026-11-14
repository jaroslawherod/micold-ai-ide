# Contract: MCP rows and arguments

Same style as `specs/551-session-spawn-tree/contracts/mcp-origin.md`.

- `create_worktree`: new optional input `parent` (a worktree ref: `default` or a worktree's directory name). Omitted: the caller's worktree (`default` for a root session; none if it cannot be resolved). Not a live worktree of the project: `invalid_input`, nothing created. Result row gains `parent` and `parent_state`.
- `list_worktrees`: rows gain `parent` (ref form; for a removed parent its last name) and `parent_state` (`"known"` | `"removed"`); both omitted when unparented. New optional input `parent` filters to direct children; `default` allowed; an unknown name returns an empty list, not an error.
- `list_sessions`, `get_session`: rows gain the same two fields, taken from the session's worktree; omitted for a session in `default` or in an unparented worktree.
- `rename_worktree`, `delete_worktree`: no argument change. Delete of a parent leaves children listed with `parent_state: "removed"`.
- No tool sets or changes a parent after creation (FR-004). Permission policy is unchanged (FR-012).
