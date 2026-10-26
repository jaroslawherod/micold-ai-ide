# Contract: one setup at a time on a sandbox daemon (#1439)

Narrows feature 1289's "the app does not serialise runs" (1289 spec.md:158, :338) for app-owned
sandboxes, because rustup corrupts or fails under concurrent installs into one home
([research R5](../research.md#r5-concurrent-mise-install-into-one-sandbox-home-spec-assumption-principle-ii)).
Spec: edge case "Concurrent setup", Principle II.

## S1. Scope

Applies when the daemon is an app-owned sandbox: it runs in its container (its data directory is
`sandbox::STATE_CONTAINER_DIR`, recorded at startup by `DaemonState::set_in_sandbox`) and has an
`offered.json` of its own (`DaemonState::offered_clis()` is `Some`, the predicate
`with_sandbox_path` uses). The first condition is needed because the host's data directory is
mounted as the container's state directory: a host daemon reads the container's `offered.json`, and
one that lists no CLI passes `offered_clis()`'s ownership check (M3 review A-F1). Host and remote
daemons run setups concurrently, as 1289 specifies.

## S2. Turn

- A daemon-wide turn (one `tokio::sync::Mutex<()>` in `DaemonState`, fair, so runs go in the order
  they asked) is taken by `spawn_run` before the setup shell is spawned and held until that run has
  ended, whatever the outcome (success, failure, stop, not started, a panic: the guard is dropped).
  A run joins the turn's queue when it is started (`start_in`), before its task runs, so the order
  is the order of the starts (M3 review A-F3).
- A run forgotten while its shell still runs (a delete whose wait for the stopped run passed
  `RUN_OVERRUN`) lets the turn go at once: nothing can **Stop** it any more, so a tree that escaped
  the kill must not hold every later setup until the daemon restarts (M3 review A-F2).
- A run that has to wait is `Running` (no new state, no protocol change) and its output starts with
  one line: `Waiting for another setup on this daemon to finish…`. The line goes through the same
  buffer, log and `SetupOutput` path as the shell's lines. A run that gets the turn at once prints
  nothing extra.
- A setup that hangs holds the turn until it ends: every waiting run's line names that cause
  (another setup on this daemon), and **Stop setup** on the running setup ends it and releases the
  turn, so the next waiting run spawns (S3).
- Only the shell runs inside the turn; record writes, pushes and the create answer never wait for it
  (1289 FR-013, FR-014 unchanged).

## S3. Ending a waiting run

- **Stop setup** on a waiting run ends it `Stopped` at once, without spawning and without waiting
  for the turn.
- A worktree delete (`stop_and_wait`) and a daemon stop end a waiting run the same way they end a
  running one (`Stopped` / `Interrupted`), at once.
- A waiting run whose turn comes after it was stopped does not spawn.
- Stopping the run that holds the turn (Stop setup, `stop_and_wait`) ends its shell as 1289
  specifies; the guard drops when that run has ended, and the first waiting run takes the turn and
  spawns.

## S4. What does not change

Setup's environment, `PATH`, shell, output bound, persistence and messages (1289 contracts C1–C2,
toolchain-path) are unchanged. Sessions are never blocked by a setup turn.
