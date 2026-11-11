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
