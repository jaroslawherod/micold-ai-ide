# Drift, milestone M1

- T004, T007, T011, T012, T020: the tests live in `crates/micold-daemon/tests/mcp/mcp_worktree_parent.rs`
  (one module of the existing `mcp` test binary, #1175), not in new top-level files, and the
  catalog scenarios (T004) share that module. Reason: a new top-level `tests/*.rs` costs a linked
  executable each.
- T011 relocation case: per-project state is keyed by the project path, so there is no relocation
  fixture to reuse; ids and links survive store round trips and a daemon restart, both tested.
- `crates/micold-client/tests/feature_write_isolation.rs`: owner rows for the new `Workspace`
  members (`worktree_ids`, `worktree_parents`), required by the guard.
- `crates/micold-core/src/store.rs`: `#[allow(clippy::large_enum_variant)]` on the transient
  `ProjectStateLoad` (the larger state struct tripped clippy).
