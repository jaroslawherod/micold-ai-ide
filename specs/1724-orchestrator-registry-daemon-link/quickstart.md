# Quickstart: verify #1724

## Part A: automated

```sh
mise run test-core                       # registry, announce, liveness, store rules
scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator
mise run gate                            # before pushing
```

## Part B: by hand (no UI in this issue)

1. `micold-daemon orchestrator` with a scratch `MICOLD_DATA_DIR`; the process stays up and spawns no
   session (`ps` shows no PTY children).
2. Link a test daemon (the integration suite's fake daemon, run from `tests/orchestrator/`); the
   listing shows it `online` with all fields.
3. Stop its replies for 10 s: `offline`; resume: `online`.
4. Link with `PROTOCOL_VERSION - 1`: refused naming both numbers, listed `needs-update`.
5. Stop and restart the orchestrator on the same directory: every daemon listed `offline`.
6. Corrupt `orchestrator/registry.json`: the start fails naming the file, the file is unchanged.
7. `micold-daemon` with no argument serves as before.

## Results (M6, 2026-11-14, cloud thread, debug build with `--features test-env`)

Part A
- `cargo test -p micold-core --all-targets --no-fail-fast`: 2474 + other binaries pass; 2 failures
  (`settings_refuses_save_over_failed_read::a_save_over_an_unreadable_file...`,
  `a_directory_that_cannot_be_emptied_names_what_survived`) are permission tests that cannot fail
  under the container's root user; neither touches the orchestrator.
- `cargo test -p micold-daemon --test orchestrator`: 39 passed, three runs in a row, including
  `no_outbound` and `netns` (the `unshare -rn` run executed, not skipped).
- `cargo fmt --check` and `cargo clippy -p micold-daemon --all-targets -D warnings` clean. `mise` is
  not installed here: CI runs `mise run gate`.

Part B (steps 2-5 are the integration suite's fake daemon, the same code path an operator's test daemon takes)
1. Run by hand: `micold-daemon orchestrator` stayed up, bound `orchestrator.sock`, no child process.
2. `link_and_list`: listed `online` with every field.
3. `heartbeat`: `offline` past the deadline, `online` on the next frame.
4. `version_mismatch`: refused naming both numbers, listed `needs-update`.
5. `restart`: every daemon listed `offline` after a restart.
6. Run by hand: corrupt `registry.json` fails the start (exit 1, `orchestrator registry <path> is
   unreadable: ...`), file unchanged.
7. `dispatch`: no argument is the daemon.
