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

## M3, batched (T030-T038)

Deviation: as M1 client and M2, the reducer and removal-flow tests (`daemons_feature.rs`, `daemon_removal.rs`) were written in one pass with `features/daemons.rs` and no separate red run was recorded; the tests were written first in commit 15707b1b, against `features::daemons` APIs that did not exist. The geometry gates (`daemons_section_geometry.rs`) went red on a real defect first (the add form's refusal and the removal dialog were not painted: the former because the draft failed on the container name before the duplicate-name check, the latter because the dialog is a floating surface outside `ui::view`), then green after the test was corrected.
- Green: `cargo test -p micold-client --test daemons_feature --test daemon_removal --test daemons_section_geometry` (9, 8, 5 tests), exit 0.
- Refactor: a stopped, removed or restarted daemon is torn down client-side through `daemon_sync::on_disconnected` (review A F1); the old `settings/daemon.rs` stays as the "Session service" section because it carries the sandbox settings.

## M4, batched (T040-T045)

Deviation: the e2e targets and the harness additions were written together; the tests are the deliverable, so there is no production code to turn green. The first runs were red for harness reasons that were then fixed, each recorded here.
- Red 1: `sandbox_real_multi_daemon` on podman, the container daemon died on its first state write (SELinux-enforcing host); `sandbox_real_staleness` and `sandbox_real_parity` fail the same way on this host before this change. Fixed in `start_sandbox` by `--security-opt label=disable` for podman only.
- Red 2: runtime CLI calls after a host daemon was spawned found no container, because the target points `XDG_RUNTIME_DIR` and `XDG_DATA_HOME` at private directories and rootless podman keeps its state there. Fixed with `runtime_cmd`, which restores the originals.
- Green: `MICOLD_TEST_RUNTIME=podman cargo test --release -p micold-daemon --features sandbox-real-runtime --test sandbox_real_multi_daemon_container --test sandbox_real_multi_daemon_host --test sandbox_real_multi_daemon -- --test-threads=1`: 3 passed. `cargo test -p micold-client --test daemon_rows_uniform`: 2 passed.
- Mutation check of the leak assertion: not run; the assertion folds each connection's real outcome through `DaemonStates`, so a leak between daemons is a wrong state on the other entry.
- Review A (round 1) found the staged harness did not compile (`HostDaemon` lacked its `socket` field), that the 5 s budget started after `stop()` returned, and that a failed spawn leaked the host daemon; all fixed (guard built before the spawn, timer taken before `stop()`, the stand-in echoes with `printf`). The three targets were re-run green after the fixes (3 x 1 passed, podman, `MICOLD_TEST_RUNTIME=podman`).
- Local run note: `mise run test-sandbox` without `MICOLD_TEST_RUNTIME=podman` fails every container test on this rootless-podman host (pre-existing `sandbox_real_*` included); CI's docker runner is the reference.
