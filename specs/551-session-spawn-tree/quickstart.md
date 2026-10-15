# Quickstart: validating the session tree

Prerequisites: `mise trust`; a project with a worktree; the `:dev` daemon (`mise run app`).

## Part A: automated

1. `mise run test-core` : origin validation, `SessionTree`, store round trip, provider fixtures,
   protocol and MCP shape tests.
2. `cargo test -p micold-daemon --test mcp_create_session --test catalog_adoption --test session_origin_removal` via
   `mise run test`: origin rows, restart survival, removed origin.
3. `cargo test -p micold-client --test sidebar_tree --test layout_snapshot`.
4. `mise run gate` before push.

## Part B: visual pass (`visual-pass` skill)

1. Start session A in a worktree, then stop the daemon and seed children (until M3's readers
   exist nothing sets an origin, and feature 776 refuses a second `create_session`): in
   the project's state file (`<data_dir>/<project_id>.json`, `JsonFileStore::project_state_path`; not
   `projects.json`, whose `sessions` is only a one-time migration seed), under `sessions`, append two copies of A's entry, each with a new
   `id` (a UUID) and `"origin": "<A's id>"`. Start the daemon. Expect: the two sessions are nested
   under A with the child marker; A has no marker. The seeded children are not live.
2. Seed a grandchild the same way (`origin` = a child's id). Expect depth 3 (a provider-reported
   grandchild still nests, FR-003).
3. Fold A. Expect: children hidden, count shown. (The attention rollup needs a live child: covered
   by the reducer test, not by this pass.)
4. Restart the app and the daemon. Expect: same tree, A still folded.
5. Delete A. Expect: the children stay, top-level, marked "origin removed".
6. Start a session from the app. Expect: top-level, no marker.
7. Repeat steps 1 and 3 under the Default entry. Restart the daemon: the seeded children are still
   nested (restart survival of a live child is the daemon test `catalog_adoption`).
8. A provider without a native signal (Copilot, Pi): sessions listed as before; the user guide
   table names the gap.
