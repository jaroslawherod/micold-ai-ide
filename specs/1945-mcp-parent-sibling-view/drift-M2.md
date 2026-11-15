# Plan drift, M2

| Path | Reason |
|---|---|
| `crates/micold-core/tests/mcp_policy.rs` | T007 adds `Operation::{Neighbourhood, SendParentInput}` to `policy::decide`; their decisions are tested here (Neighbourhood proceeds as a read; SendParentInput decides as SendSessionInput at every option value). |
| `crates/micold-daemon/tests/mcp/mcp_worktree_parent_visibility.rs` | Its `Fixture`, constants and helpers are made `pub(crate)` and gain `remove_worktree`, so `mcp_worktree_parent_neighbourhood.rs` reuses the same fixture instead of copying it (duplication budget). Its removed-worktree test uses the new helper. |
| `crates/micold-daemon/tests/mcp_audit_log.rs` | Iterates every mutating tool from `tools/list`: `send_parent_input` needs a good and a failing call. The fixture gains a root session (the caller's parent `default`), started by `prepare`; `expected_target` reads `parent` for a call with only a text. |
| `crates/micold-daemon/tests/mcp_binding_spawn.rs` | Asserts the full `tools/list` names: adds `neighbourhood` and `send_parent_input`. |
| `crates/micold-daemon/tests/mcp_read_latency.rs` | Times every read-only tool from `tools/list` and panics on one it has no arguments for: adds `neighbourhood`. |

Catalog order: `send_parent_input` is listed after `report_review_comment`, not beside `send_session_input`, because an existing contract test (`report_review_comment_is_listed_after_send_session_input_as_the_contract_gives`) pins the report tool right after `send_session_input`.

`parent_target` takes `(id, worktree dir, lifecycle)` triples rather than `SessionSummary` rows, so its unit test needs no full summary; `None` is `default`, so a live directory literally named `default` never matches the project root.
