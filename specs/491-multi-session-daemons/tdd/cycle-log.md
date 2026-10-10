# Cycle log: 491-multi-session-daemons

Append only.

## Baseline

Commit 52c691b8 (origin/main). Profile baseline: suite green (5071 passed). Full suite not re-run at planning; `mise run test-core` is the loop's inner check.

## M1 core, batched cycles (U1-U22)

Deviation: cycles were batched per test file (registry, migration, bindings, isolation), not one behavior each.
- Red: `scripts/build-lock.sh cargo test -p micold-core --no-fail-fast --test daemons_registry --test daemons_migration --test daemons_bindings --test daemons_isolation` against `todo!()` stubs: 23 tests failed (panics at daemons.rs `not yet implemented`; migration/bindings by assertion on empty `daemons` and unpersisted `bindings`).
- Green: same command, exit 0 after implementing daemons.rs, settings migration, store bindings, Placement::from_runtime.
- Refactor: none needed. Whole-core regression run pending (see ledger Handover).
