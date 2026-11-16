# Plan drift, M2

| Path | Reason |
|---|---|
| `crates/micold-core/tests/mcp_policy.rs` | T007 adds `Operation::{Neighbourhood, SendParentInput}` to `policy::decide`; their decisions are tested here (Neighbourhood proceeds as a read; SendParentInput decides as SendSessionInput at every option value). |
| `crates/micold-daemon/tests/mcp/mcp_worktree_parent_visibility.rs` | Its `Fixture`, constants and helpers are made `pub(crate)` and gain `remove_worktree`, so `mcp_worktree_parent_neighbourhood.rs` reuses the same fixture instead of copying it (duplication budget). Its removed-worktree test uses the new helper. A link parent `"default"` records the project root (`ParentChoice::Default`), as the ref means, now that a link to a directory named `default` is told apart (review A round 1, F2). |
| `crates/micold-daemon/tests/mcp_audit_log.rs` | Iterates every mutating tool from `tools/list`: `send_parent_input` needs a good and a failing call. The fixture gains a root session (the caller's parent `default`), started by `prepare`; `expected_target` names that session for `send_parent_input` (review A round 1, F4), and a new test checks the line names the chosen session and `parent`, and that the text is never logged, delivered or refused. |
| `crates/micold-daemon/tests/mcp_binding_spawn.rs` | Asserts the full `tools/list` names: adds `neighbourhood` and `send_parent_input`. |
| `crates/micold-daemon/tests/mcp_cross_session.rs` | Review A round 1 (F1, F2): `Sandbox::with` adds sessions to the catalog (a shell ahead of the parent's AI CLI session); two delivery tests, a shell first in the parent worktree and a parent directory named `default`. |
| `crates/micold-core/src/worktree_parent.rs` | Review A round 1 (F2): `ParentView` and `ParentSide::Known` carry `root` (a link to the project root), so `neighbourhood` tells a live directory named `default` from the root by the link; such a directory lists the worktrees recorded under it as its children. |
| `crates/micold-core/src/mcp/tools.rs` | Review A round 1 (F4): the `send_parent_input` audit-target comment says the daemon names the chosen session. |
| `crates/micold-daemon/tests/mcp_read_latency.rs` | Times every read-only tool from `tools/list` and panics on one it has no arguments for: adds `neighbourhood`. |

Catalog order: `send_parent_input` is listed after `report_review_comment`, not beside `send_session_input`, because an existing contract test (`report_review_comment_is_listed_after_send_session_input_as_the_contract_gives`) pins the report tool right after `send_session_input`.

`parent_target` takes `(id, worktree dir, lifecycle)` triples rather than `SessionSummary` rows, so its unit test needs no full summary; `None` is `default`. Its caller passes only AI CLI sessions and derives `None` from the link's `root`, not from the name, so a live directory literally named `default` never matches the project root (review A round 1, F1, F2).

`send_parent_input` resolves caller, project and target from one snapshot and hands them to `deliver_input`, the part of `send_session_input` after its resolution (review A round 1, F5), instead of calling `send_session_input` and resolving twice.

`MISSING contracts/mcp-neighbourhood.md`: reference only; the code follows it and it needed no change.

| Path | Reason |
|---|---|
| `crates/micold-daemon/src/mcp/tools.rs` (`Scope`) | Review A round 2 (F1): the scope keeps the project root apart from the worktree directory names (`Visible { root, dirs }`, `allows_root`/`allows_dir`/`allows_ref`/`allows_place`), so a directory named `default`, as the caller's own or its recorded parent, never grants the root's sessions. F3: a parent running only shells is refused as having "no running AI CLI session (only shell sessions)". F4: `send_parent_input` runs the `Scope::session` check on its chosen target. |
| `crates/micold-daemon/tests/mcp/mcp_worktree_parent_neighbourhood.rs` | Review A round 2 (F1): `a_directory_named_default_never_grants_the_project_roots_sessions`, a caller in a directory named `default` and a caller whose parent is that directory, with a root session present. |
| `crates/micold-daemon/tests/mcp_cross_session.rs` (round 2) | Review A round 2 (F3): `a_parent_running_only_a_shell_is_refused_as_having_no_ai_cli_session`; the shell setup is shared as `with_a_running_shell_in_the_parent`. `a_parent_directory_named_default_is_not_the_project_root` now starts the root session with the full view before linking, as the root is outside the linked caller's scope (F1). |
| `specs/1945-mcp-parent-sibling-view/contracts/mcp-neighbourhood.md` | Review A round 2 (F3): a refusal row for a parent running only plain shell sessions. |
