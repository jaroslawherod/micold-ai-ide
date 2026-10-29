# Quickstart: validating #1602

Prerequisites: `mise trust`, a scratch git project with one remote-less worktree `w1`, the tool server
enabled in Settings, a session running in `w1` (any CLI that can call MCP tools, or the daemon test
client in `crates/micold-daemon/tests/mcp/`).

## Part A: automated

1. `mise run test-core` (policy table, parsers, wire round trip, schema pin).
2. `mise run gate` (daemon integration, client, formatting, clippy).
3. On a Windows runner: the daemon test that holds a handle in W (SC-006).

## Part B: visual and manual pass (needs eyes at a display; `visual-pass` skill)

1. Make `w1` dirty: edit a tracked file, add an untracked file, commit once without pushing.
2. From the session in `w1`, call `delete_worktree` with `worktree: "w1"`, `stop_sessions: true`.
   Expect: a prompt in every window naming the session, the operation and `w1`; the changed files and the
   unpushed commit listed; a line saying the asking session ends. Contract: [mcp-tools.md](./contracts/mcp-tools.md).
3. Decline. Expect the agent to see "declined by the user" and `w1`, its branch and the session unchanged.
4. Repeat, wait 60 s: "needs confirmation", nothing changed.
5. Make `w1` clean and pushed, repeat, accept. Expect: the agent receives "scheduled", then within 10 s the
   session is Idle and `w1` and its branch are gone from the sidebar and from disk.
6. Add 60 changed files; the prompt shows 50 and "10 more".
7. Force a failure (make a file in `w1` immutable on Linux, or hold a directory open on Windows); accept.
   Expect a notice naming `w1` and the reason, and a daemon log error.
8. From a Default session call `delete_worktree`: refused, no prompt. From any session call `stop_session`
   with no id: prompt names the session as caller and target; accept: result first, then it stops.
