# Quickstart: validating the session tree

Prerequisites: `mise trust`; a project with a worktree; the `:dev` daemon (`mise run app`).

## Part A: automated

1. `mise run test-core` : origin validation, `SessionTree`, store round trip, provider fixtures,
   protocol and MCP shape tests.
2. `cargo test -p micold-daemon --test mcp_create_session --test catalog_adoption --test session_origin_removal` via
   `mise run test`: nesting through `create_session`, restart survival, removed origin.
3. `cargo test -p micold-client --test sidebar_tree --test layout_snapshot`.
4. `mise run gate` before push.

## Part B: visual pass (`visual-pass` skill)

1. Start session A in a worktree. From A, call the tool server's `create_session` for the same
   worktree. Expect: the new session is nested under A with the child marker; A has no marker.
2. From the new session call `create_session` again. Expect depth 3.
3. Fold A. Expect: children hidden, count shown; make a child need attention: A shows it.
4. Restart the app and the daemon. Expect: same tree, A still folded.
5. Delete A. Expect: the children stay, top-level, marked "origin removed".
6. Start a session from the app. Expect: top-level, no marker.
7. Repeat steps 1 and 3 under the Default entry. Kill the daemon while a child runs; after restart
   the child is resumable and still nested.
8. A provider without a native signal (Copilot, Pi): sessions listed as before; the user guide
   table names the gap.
