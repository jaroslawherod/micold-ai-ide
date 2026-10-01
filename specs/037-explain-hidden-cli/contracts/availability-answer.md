# Contract: The availability answer carries the environment state

Research: [R1–R3, R5](../research.md). Types: [data-model.md](../data-model.md).

## A1 — Wire

```text
ClientMsg::AiCliAvailabilityRequest { req, cwd }            unchanged
DaemonMsg::AiCliAvailability { req, available, env }        env: Option<SpawnEnv>  (new)
PROTOCOL_VERSION                                            17 → 18
```

`crates/micold-core/src/protocol/messages.rs`, `crates/micold-core/src/protocol/version.rs`.
`SpawnEnv` is declared in `crates/micold-core/src/cli_reason.rs`.

## A2 — What the service answers

| # | Situation for the asked directory (home when `cwd` is `None`) | `available` | `env` |
|---|---|---|---|
| S1 | Environment-include off | walk of the service's own `PATH` | `Some(IncludeOff)` |
| S2 | On, script path blank | walk of the service's own `PATH` | `Some(NoScriptPath)` |
| S3 | On, the attempt found no script | walk of the service's own `PATH` | `Some(ScriptNotFound)` |
| S4 | On, the attempt exited with an error (this includes a script path that exists but cannot be sourced: a directory, a file the user may not read; FR-001) | walk of the service's own `PATH`, unless the attempt returned one | `Some(ScriptFailed)` |
| S5 | On, the attempt timed out | walk of the service's own `PATH`, unless the attempt returned one | `Some(ScriptTimedOut)` |
| S6 | On, the attempt succeeded | walk of the resolved `PATH` | `Some(Applied)` |
| S7 | No directory can be resolved (no `cwd`, no home), or the resolving task failed | `available_here()`, as today | `SpawnEnv::classify(enabled, path, None)`: `Some` for S1 and S2's settings, else `None` |

`available` is computed exactly as before this feature in every row.

## A3 — One attempt, one answer (FR-012, FR-014)

- `available` and `env` are read from one `ResolvedEnv`, returned by one call of
  `DaemonState::spawn_env_for(cwd)`. `ai_clis_available_in` becomes
  `availability_in(cwd) -> (Vec<AiCli>, SpawnEnv)`, and the old name stays as the first half for
  callers that need only the set.
- The launch gate (`state.rs`, the `plan.mode == TerminalMode::AiCli` check in the start path) and
  the tool server's `create_session` check read the same `ResolvedEnv` for their directory, so a
  failure and an answer about one attempt cannot disagree.
- Asking for a reason runs no script. With a filled cell, `spawn_env_for` returns the cell. The
  number of script runs is the number before this feature: one per directory per invalidation.
  `crates/micold-daemon/tests/ai_cli_availability.rs`
  (`a_second_answer_for_a_directory_does_not_run_the_script_again`) and
  `tests/env_include_cache_coherence.rs` count runs with a script that appends to a file, and keep
  passing.
- Invalidation is unchanged: `set_env_include` clears every cell when an env-include field
  changes, and `invalidate_env_include` removes one directory's.

## A4 — What the client does with it

| # | Rule | Site |
|---|---|---|
| C1 | The shell builds `CliAvailability { available, source, env, .. }` from the message and the boot plan, as today for `source` | `shell/daemon_sync.rs`, the `DaemonMsg::AiCliAvailability` arm |
| C2 | `AvailabilityAnswers::answered(req, answer)` sets `asked_for` to the key the request named, then files it under that key. An answer that is not the newest for its key is dropped, as today | `features/session.rs` |
| C3 | Readers get `env` and `asked_for` with `available`, from `home()` and `for_dir()`. No reader takes them from anywhere else | `features/session.rs`, `features/settings.rs` |
| C4 | No new request is sent. The closed list of askers (033 contract C1) is unchanged, and `tests/availability_is_asked_only_on_named_events.rs` keeps passing | — |

## A5 — Older and newer peers

A client and a service that differ in `PROTOCOL_VERSION` do not exchange this message: the
handshake refuses first, and 027's recovery applies. No compatibility shim is added.
