# Cycle log: 491-multi-session-daemons

Append only.

## Baseline

Commit 52c691b8 (origin/main). Profile baseline: suite green (5071 passed). Full suite not re-run at planning; `mise run test-core` is the loop's inner check.

## M1 core, batched cycles (U1-U22)

Deviation: cycles were batched per test file (registry, migration, bindings, isolation), not one behavior each.
- Red: `scripts/build-lock.sh cargo test -p micold-core --no-fail-fast --test daemons_registry --test daemons_migration --test daemons_bindings --test daemons_isolation` against `todo!()` stubs: 23 tests failed (panics at daemons.rs `not yet implemented`; migration/bindings by assertion on empty `daemons` and unpersisted `bindings`).
- Green: same command, exit 0 after implementing daemons.rs, settings migration, store bindings, Placement::from_runtime.
- Refactor: none needed. Whole-core regression run pending (see ledger Handover).

## M1 client, batched (U23-U24, A1-A3)

Deviation: tests and implementation were written in one pass without a separate red run (context was spent on the build lock); the tests were written before the code they exercise and each asserts on behaviour that did not exist (`daemon_label_of`, `NO_DAEMON_LABEL`, `registry_loaded`, form `daemons`/`daemon`/`explain_block`, `Workspace::bind`), so they could not compile, let alone pass, before the change.
- Green: `cargo test -p micold-client --test worktree_daemon_label --test upgrade_single_daemon --test worktree_form_daemon_choice --test features_sidebar --test sidebar_tree`: exit 0.
- Core: `mise run test-core` green apart from `github_locate_desktop_launch` (environment: `gh` only on the mise path, #696 / PR #750; skipped locally with `MICOLD_SKIP_GH_LAUNCH_TEST=1`).
- Refactor: none needed.
