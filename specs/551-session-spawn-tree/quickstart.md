# Quickstart: validating the session tree

Prerequisites: `mise trust`; a project with a worktree; the `:dev` daemon (`mise run app`).

## Part A: automated

1. `mise run test-core` : origin validation, `SessionTree`, store round trip, provider fixtures,
   protocol and MCP shape tests.
2. `cargo test -p micold-daemon --test mcp_create_session --test catalog_adoption --test session_origin_removal` via
   `mise run test`: origin rows, restart survival, removed origin.
3. `cargo test -p micold-client --test sidebar_tree --test sidebar_spawn_tree --test layout_snapshot`.
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

## Results (recorded at close, 2026-10-18)

Part A: the commands above ran green in each milestone's gate (`mise run gate`).

Part B, screenshots in `visual/` from the M2 and M4 `visual-pass` runs:

| Step | Result | Evidence |
|---|---|---|
| 1 | PASS: two seeded children nested under A with the child marker, A unmarked | `visual/c1.png` |
| 2 | PASS: grandchild nests at depth 3 | `visual/c-grand.png` |
| 3 | PASS: A reads "claude, 2 agents", children hidden | `visual/c-fold.png` |
| 4 | PASS: tree and fold survive a restart | `visual/c-fold-restart.png` |
| 5 | PARTIAL: the screenshot shows the children top-level with the "origin pending" marker (the seeded origin never appeared); "origin removed" is proven by tests only (`session_origin_removal.rs`, `sidebar_spawn_tree.rs`), not recorded visually | `visual/c-orphan.png` |
| 6 | By test (not recorded visually): an app-started session has no origin, so no marker | covered by `a_child_row_acts_like_a_top_level_row` and layout gates |
| 7 | PASS: same under the Default entry | `visual/c-default.png` |
| 8 | PASS by test: `provider_origin_docs` checks the user-guide table names the gap | `provider_origin_docs.rs` |

Cosmetic note: a folded row shows a folder icon where the twisty was.
