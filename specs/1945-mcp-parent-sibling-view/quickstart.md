# Quickstart: 1945

Daemon and MCP only; there is no GUI pass (no §B).

1. `scripts/build-lock.sh cargo test -p micold-core worktree_parent` (derivation).
2. `scripts/build-lock.sh cargo test -p micold-daemon --test mcp worktree_parent` (visibility,
   neighbourhood, parent messaging, record-before-start).
   `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_cross_session` (delivery to the
   parent, record-before-start with a real stub session).
3. Manual: in a project with a hand-made worktree `A`, run `list_sessions` from its session: `default`
   sessions appear. Call `neighbourhood`: `parent` is `default`, `recorded: false`. Call
   `send_parent_input` with a text: the `default` session receives it.
4. Before pushing: `scripts/gate.sh`.
