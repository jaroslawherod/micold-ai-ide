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

## M2, batched (T018-T029, T048-T049)

Deviation: as M1 client, tests and implementation were written in one pass across two units without a separate recorded red run; the test files were written before the code they exercise (`daemon_state.rs` first, against `DaemonState`/`DaemonStates`, then the client targets) and each asserts behaviour absent on `origin/main` (no per-daemon links, routing by binding, labels or state machine existed there; the files do not compile against it).
- Green: `cargo test -p micold-core --test daemon_state` (11) and `cargo test -p micold-client --test per_daemon_actors --test daemon_routing --test catalog_two_daemons --test daemon_version_mismatch --test worktree_unavailable_label --test daemon_adopt_on_restart`, exit 0 (review B reran them); binary tests `two_daemons` (op routed to the bound daemon only; one daemon dropping leaves the other's connection and pending ops).
- Refactor: `links` and `runtime_missing` live in `settings::State`; `Message::Daemon` folded into `connection::Msg::OfDaemon` so the root vocabulary stays cross-cutting (`root_vocabulary_is_cross_cutting`).
