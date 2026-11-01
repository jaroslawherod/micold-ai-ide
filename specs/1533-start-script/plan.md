# Implementation Plan: An optional start script prepares each daemon

**Branch**: `feat/1533_an-optional-start-script-prepares-each-daemon` | **Date**: 2026-11-01 |
**Spec**: [spec.md](spec.md)

**Input**: `specs/1533-start-script/spec.md`; parent design `specs/daemon-config-file/` (research D12,
contract `[startup]`, parent tasks T068 to T077).

## Summary

`micold-daemon` runs the user's script itself, in `main` after the configuration is read and before the
runtime exists, so every start path (host spawn, SSH bridge, container entrypoint, pod) runs one
implementation. The script is sourced in a disposable shell by a generalised `env_include::run_and_diff`;
the explicit exports are applied to the daemon's environment, which sessions inherit, so the fixed
precedence needs no new merge code. The run record is `<data_dir>/startup.done`, the outcome is
`<data_dir>/startup-result.json`, a serving daemon reports it in `ConfigStatus.startup`, and a refused
daemon prints it to stderr and the start path reads the file. Settings → Daemons gets a "Start
script" group; an inline script is saved as an application-owned file and delivered like a picked
file. Decisions and rejected options: [research.md](research.md).

## Technical Context

**Language/Version**: Rust (workspace edition), iced for the client.
**Primary Dependencies**: existing only: `micold-core` (`env_include`, `process`, `daemon_config`,
`protocol::hashing`), `micold-daemon` (`config_load`, `config_watch`, `main`), `micold-client`
(`daemon_config_render`, `ui/settings/daemons.rs`), `micold-material` (`TextArea`). No new crate.
**Storage**: files under the daemon's data directory (`startup.done`, `startup.lock`,
`startup-result.json`, `startup-env.json`); the inline script as a file under the application's data dir.
**Testing**: `cargo test` through `mise run test-core` (core), `mise run gate`; daemon integration
tests in `crates/micold-daemon/tests/startup_script.rs`; client tests in `micold-client`; geometry
gates for the Settings group; quickstart Part B for the visual pass; `mise run test-sandbox` for
container delivery.
**Target Platform**: Linux, macOS, Windows (PowerShell variant); container and pod daemons are Linux.
**Project Type**: desktop app + daemon (workspace of four crates).
**Performance Goals**: none new; the script is bounded by `timeout_secs` (default 300 s).
**Constraints**: nothing listens during the run (SC-004); no secret value in any file or log the
daemon writes beyond the script's own output; works offline (Principle IV).
**Scale/Scope**: one script per daemon; output tail 200 lines.

## Constitution Check

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every milestone starts with its failing tests (parent T068 to T071 are the core ones); `speckit-tdd-plan` derives the list from the requirements below. |
| II. Multi-Session | PASS | State is per data directory with a lock; a second start waits; sessions inherit the daemon's environment and share nothing mutable. |
| III. Worktree Integration | PASS | The script runs with the daemon's data directory as its working directory, not a worktree; no file or VCS operation is added. |
| IV. Local-First | PASS | All state is local files; the script may use the network only because the user wrote it; the app sends nothing anywhere. |
| V. Rust + iced | PASS | `StartupScript` is an enum (File \| Inline); `StartupReport.outcome` is an enum with the reason typed; validation refuses invalid values before they reach the plan. |
| VI. Cross-Platform | PASS with documented limit | PowerShell shell, `LockFileEx` lock, `win_job.rs` kill; exports capture is unavailable for a shell that is neither POSIX nor PowerShell (stated in docs). CI covers all three. |
| VII. Documentation | PASS | `docs/user-guide/daemon-configuration.md` and `sandboxed-daemon.md` in the docs milestone (FR-012); `docs/daemon.md` for the files. |
| VIII. Reusable UI | PASS | `TextArea` for the inline editor, the existing path-picker; a missing building block is added to `micold-material` with the builder API. |

## Complexity Tracking

None: no justified violation.

## Project Structure

### Documentation (this feature)

```text
specs/1533-start-script/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md
├── contracts/startup-run.md
├── checklists/
└── tasks.md            # the tasks unit
```

### Source code (new paths marked +)

```text
crates/micold-core/src/
├── env_include.rs            # run_and_diff (generalise resolve); env_file wrapper
├── daemon_config/            # existing Startup/validate/StartupView (no new key)
├── daemons.rs                # StartupSettings, StartupScript on the daemon entry
├── protocol/messages.rs      # StartupReport; ConfigStatus.startup
├── protocol/version.rs       # PROTOCOL_VERSION +1 (version.rs, currently 75; tests/schema_hash.rs recomputes, no literal to edit)
└── spawn.rs                  # host spawn reads startup-result.json on a failed start
crates/micold-daemon/src/
├── startup_script.rs +       # plan, hash, record, lock, result, apply exports
├── main.rs                   # run the plan between load_startup and the runtime
├── config_watch.rs           # script/env_file hash into restart_keys; status.startup
├── remote.rs                 # bridge relays the failure block and the file
└── state.rs                  # config_status carries startup
crates/micold-daemon/tests/startup_script.rs +
crates/micold-client/src/
├── daemon_config_render.rs   # render [startup]
├── ui/settings/daemons.rs    # Start script group, restart notice
├── ui/daemon_details_dialog.rs  # failure / warning with "Show output"
└── shell/daemon_sync.rs      # deliver script + env_file per kind (with #1526 to #1528)
docs/user-guide/{daemon-configuration,sandboxed-daemon}.md, docs/daemon.md
```

## Requirement map

| Requirement | Where |
|---|---|
| FR-001 keys, no inline content | existing `daemon_config` validate; render in `daemon_config_render`; research R2 |
| FR-002 before listener, own user | contract S1; `main.rs` (research R4); container user unchanged |
| FR-003, FR-004 exports and order | research R5; `run_and_diff`; precedence test (parent T070) |
| FR-005 once_per_container | research R1; data-model *Run record* |
| FR-006 timeout | watchdog + `StreamingChild::terminator()` (new streaming variant of `run_and_diff`); contract S2 step 4 |
| FR-007 refuse / warn | contract S2 step 6 |
| FR-008 output and tail on every path | research R3; contract S3; `ConfigStatus.startup` |
| FR-009 no re-run, `restart_needed` | research R6; contract S4 |
| FR-010 Settings | research R8; `ui/settings/daemons.rs` |
| FR-011 delivery per kind | research R2; `daemon_sync.rs`; #1526 to #1528 |
| FR-012 docs | docs milestone |

## Test strategy

| Behaviour | Layer |
|---|---|
| `run_and_diff` exports, newline value, `env_file` literal and unexported, timeout, backgrounded child | core unit (`env_include.rs`) |
| Hash, record write-after-success, lock, tail bounds, result JSON | core/daemon unit (`startup_script.rs`) |
| Runs before bind, no connection accepted (SC-004), refuse vs warn, once vs every, reload never re-runs | daemon integration (`tests/startup_script.rs`) |
| Precedence order and `MICOLD_*` last (SC-003) | daemon integration with a spawned session |
| `ConfigStatus.startup` round trip, version bump | core protocol tests |
| Render `[startup]`, validation refusals before delivery, inline script saved as file | client unit (`daemon_config_render`, settings state) |
| Failure/warning shown with tail, restart notice | client tests; geometry gates for the group; quickstart Part B |
| Delivery of script and `env_file` per kind | client tests with fakes; `mise run test-sandbox` (container); cluster check in quickstart |

## Milestones

Ordered by what needs no open prerequisite first; the per-kind delivery and error-display milestones
land as each of #1526, #1527, #1528 merges (parent issues 5, 6, 7). If a prerequisite is still open
when its turn comes, the milestones after it that do not depend on it go first.

| M | Deliverable | Needs |
|---|---|---|
| M1 | Core: `run_and_diff` (+ `resolve` as caller), `env_file` wrapper, `StartupReport`, `ConfigStatus.startup`, protocol bump | #1524 (closed) |
| M2 | Daemon: `startup_script.rs`, `main.rs` wiring, record/lock/result, exports, precedence, reload keys, `config check` unchanged; all in integration tests (US1 to US3, SC-003 to SC-005) | M1 |
| M3 | Application model and Settings: `StartupSettings`, render, validation, Start script group, inline editor, restart notice, inline file storage (US5.1 to US5.3) | #1524 (closed) |
| M4 | Host and SSH: delivery of script and `env_file`, failure/warning display, host spawn and bridge read the result (US4, US5.4 on H and S) | M2, M3, #1526 |
| M5 | Container: delivery under `/run/micold/config` and `/run/micold/secrets`, entrypoint start path, error display | M2, M3, #1527 |
| M6 | Pod: delivery through the volumes, error display | M2, M3, #1528 |
| M7 | Docs (FR-012) and the quickstart run | M2 to M6 as merged |

M1 and M3 are independent and can be done in either order; M2 follows M1. M7 is also split if a kind
is deferred: the guide states which kinds are delivered.

## Risks

- Writing the log before the tracing subscriber exists: the run writes to the log file and stderr
  directly (research R4); tested by reading the log after a run.
- A start path that discards the daemon's stderr (a detached Windows daemon): the result file is the
  source of truth; the stderr block is only a courtesy.
- `PROTOCOL_VERSION` conflicts with the three open delivery issues: each bumps it too; whoever merges
  second rebases and takes the next number (recorded in the ledger at that time).
