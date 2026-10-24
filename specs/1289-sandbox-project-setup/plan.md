# Implementation Plan: Per-project setup command and PATH for project toolchains

**Branch**: `claude/project-thread-d7q05f` | **Date**: 2026-10-24 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1289-sandbox-project-setup/spec.md`

## Summary

There are two parts.

**(1) PATH.** On an app-owned sandbox (a container or a pod, recognised by its `offered.json`),
the one seam that already puts the offered AI CLIs in front of a spawn's `PATH`
(`DaemonState::with_offered_path`, reached through `spawn_env_for`) also adds two folders, in this
order: the sandbox home's `~/.local/bin`, then the mise shims folder. Duplicates are dropped. A
pure core function computes the order. Sessions, terminals, MCP servers (through the AI CLI's
environment), the availability check and the setup command all get it from that one place.

**(2) Setup command.** Each project has an optional setup command. It is stored in the app's
`settings.json` and edited in a new Settings › **Project** section. The client hands it to every
daemon that holds the project, in memory (`ProjectSetupSet`):
- on every connect;
- on every change.

Every app create, which all go through `ops::create_worktree`, starts it on the worktree's daemon
right after the worktree exists. **Run setup** starts it again on a worktree or the Default entry.

A run belongs to the daemon:
- **Process**: a non-PTY child in its own process group or job, with stdin at EOF and the
  session's environment.
- **Output**: captured into a 10,000-line buffer and a log file.
- **Record**: persisted so that a daemon stop leaves *Interrupted*.

Clients see the state as a `SetupChanged` push. A row `Tag` shows it. The menu gains **Setup
output**, **Run setup** and **Stop setup**. A `ScrollingDialog` streams the output through a
per-connection watch. Each client sends one error notification per failed run. MCP
`create_worktree` reports `"setup": "started" | "none" | "already running"` (C4).

The per-project image is out of scope (clarify Q2). Decisions and rejected alternatives:
[research.md](research.md).

## Technical Context

**Language/Version**: Rust stable (mise-managed).

**Primary Dependencies**: Existing only, and no new crates:
- `serde`, `serde_json`, `tokio` (daemon) and `iced` (client);
- `libc` (Unix group kill) and `windows-sys` (job object), both already used by
  `micold_core::process`.

**Storage**:
- `settings.json` gains a `setup_commands` map (client-written, kept by the host daemon's
  service-settings write).
- The daemon's data dir gains `setup/<project_id>/<location>.{json,log}`, with the run record and
  the output tail. Everything is local (Principle IV).

**Testing**: The layers are:
- `cargo test -p micold-core`: path order, setup model, output buffer, shell choice, streaming
  child, settings and protocol round-trips;
- `micold-daemon` integration tests: run lifecycle over the wire, MCP result, restart, delete,
  attach catch-up;
- `micold-client` tests: reducer, menu predicates, notification dedupe, settings section;
- client geometry gates: the row tag and the output dialog;
- the real sandbox suite (`mise run test-sandbox`) for `PATH` inside the basic image;
- quickstart Part B for the visual pass and the real-CLI checks.

**Target Platform**: Linux, macOS and Windows hosts. The `PATH` change applies inside Linux
sandboxes only. Setup runs on every daemon kind: `bash`/`sh` on Unix, PowerShell on a Windows host
daemon.

**Project Type**: Desktop app, three crates (`micold-core`, `micold-daemon`, `micold-client`).

**Performance Goals**: Worktree creation is not slowed by setup by more than 1 s (SC-005): the
start is a record write plus a spawn on the blocking pool, after the create has answered. The
output view stays smooth at 10,000 lines (`VirtualRows`). Output pushes go out at most every
250 ms per watcher.

**Constraints**: No timeout on setup. Nothing waits for setup (FR-013, FR-014). Output never
reaches `settings.json`. Host and remote `PATH` are unchanged (FR-008).

**Scale/Scope**: One run per worktree at a time, tens of worktrees per project, output up to
about 10^5 lines per run, of which 10^4 are kept.

## Constitution Check

| Principle | Status | How |
|---|---|---|
| **I. Test-First** | PASS | Every decision is in tested pure logic: `toolchain_path`, `SetupRun` transitions, `OutputBuffer`, `shell_line` and `SetupCommand::parse` in core; the client's menu and notification predicates in `features/setup.rs`. Daemon behaviour is driven by integration tests over the wire (Part A). Only render glue in `src/ui/` is left to the geometry gates and Part B. |
| **II. Multi-Session** | PASS | Setup state is per (daemon, project, location), not per session. It is never put into session state or the state icon (R11). Starting a session never waits for it (FR-014), and no session's state changes because of it. |
| **III. Worktree Integration** | PASS | Setup runs only in an app-managed worktree or the Default root (`location.cwd(project)`). It starts from the app's own create path (`ops::create_worktree`), and the app stops it before it deletes a worktree (FR-024). The Default entry gets **Run setup** only, never a worktree operation. |
| **IV. Local-First** | PASS | The command is in local `settings.json`, and records and output are in the daemon's local data dir. The command itself may use the network, exactly as a session on that daemon can (edge case). The app sends nothing anywhere. |
| **V. Rust + iced** | PASS | `SetupState`, with its `Failure` variants, is a closed enum, and `can_become` is the only transition. An ended run without `ended_at` cannot be represented. `SetupCommand` cannot be blank. |
| **VI. Cross-Platform** | PASS | Core takes the platform as a value (`ShellPlatform`) and never branches on the OS. The process tree is ended through the existing group/job mechanism, shared with `run_bounded`. `StreamingChild` tests run on all three CI platforms. The `PATH` rule is joined with `join_paths`. |
| **VII. Documentation** | PASS | The same change updates the user guide: `docs/user-guide/settings.md` (Project section), `worktrees-and-sessions.md` (row tag, menu entries, output view, notification) and `sandboxed-daemon.md` (the folders on `PATH`; the pointer from "Which image it runs on"; a project's own image through a second container daemon and the remembered **Daemon** select, FR-025). `agent-tools.md` gets the `setup` result field. |
| **VIII. UI Components** | PASS | Only shared primitives are used: `Tag`, `TextArea`, `ScrollingDialog`, `VirtualRows`, `material::MenuItem`, and existing icons. No new widget, so no new builder API is needed. |

Post-design re-check: PASS. There is nothing in Complexity Tracking.

## Project Structure

### Documentation (this feature)

```text
specs/1289-sandbox-project-setup/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── toolchain-path.md
│   ├── wire-and-settings.md
│   └── setup-ui.md
└── tasks.md             # tasks unit
```

### Source Code

```text
crates/micold-core/src/
├── sandbox/toolchain_path.rs     # NEW: toolchain_path(), ToolchainPathInput (R1–R3)
├── sandbox/mod.rs                # pub mod toolchain_path
├── setup/                        # NEW
│   ├── mod.rs                    # SetupCommand, RunId, SetupState, Failure, SetupRun(+Summary), SetupStart
│   ├── output.rs                 # OutputBuffer (R7)
│   └── shell.rs                  # ShellPlatform, shell_line (R6)
├── lib.rs                        # pub mod setup
├── process.rs                    # NEW StreamingChild, Terminator, ExitKind; group kill/job shared with run_bounded
├── provider.rs                   # resolve_on_path made pub(crate) for shell_line's bash lookup
├── settings.rs                   # Settings::setup_commands (+ on-disk mirror), SetupCommandChange
├── store.rs                      # JsonFileStore::setup_dir/setup_record_path/setup_log_path/remove_setup
└── protocol/messages.rs          # ProjectSetupSet, SetupRun, SetupStop, SetupOutputWatch/Unwatch, SetupChanged, SetupOutput
    protocol/version.rs           # PROTOCOL_VERSION 69

crates/micold-daemon/src/
├── setup.rs                      # NEW SetupRuns: commands, runs, start/stop/finish/on_attach/forget/interrupt_all; run task
├── state.rs                      # with_offered_path → with_sandbox_path (toolchain_path, no early return on empty list); SetupRuns field + accessors; load records at start
├── ops.rs                        # create_worktree → Result<SetupStart, _> and start_setup; delete_worktree stop_and_wait → git remove → forget setup
├── server.rs                     # handlers for the five client messages; SetupChanged on attach; unwatch on disconnect; unwind → interrupt_all
├── runs.rs                       # follows create_worktree's new return type
└── mcp/tools.rs                  # "setup" field on the create_worktree result (C4)

crates/micold-client/src/
├── features/setup.rs             # NEW render-free reducer (data-model: client)
├── features/mod.rs               # pub mod setup
├── features/settings.rs          # SettingsSection::Project, draft.project.setup_command, follow(project)
├── shell/persist.rs              # write setup_commands; send ProjectSetupSet on change
├── shell/daemon_sync.rs          # send ProjectSetupSet after register_projects; route SetupChanged/SetupOutput; PendingOp entries
├── ui/settings/project.rs        # NEW section view (TextArea)
├── ui/mod.rs                     # location_menu_items: Setup output / Run setup / Stop setup
├── ui/sidebar.rs                 # setup Tag in the row's tag line (and Default row)
├── ui/setup_output.rs            # NEW output dialog (ScrollingDialog + VirtualRows)
└── ui/confirm_delete.rs          # "Setup is running here…" line (U6)

docs/user-guide/{settings,worktrees-and-sessions,sandboxed-daemon,agent-tools}.md
```

**Structure Decision**: Decisions go in `micold-core` (pure, cross-platform, tested). Process
supervision, persistence and the wire go in `micold-daemon`, which owns the run. `micold-client`
has a render-free reducer plus thin views built from shared components.

## Requirement map

| FR | Where |
|---|---|
| FR-001, FR-004 | `sandbox::toolchain_path` order and dedupe; applied in `state.rs::with_sandbox_path` |
| FR-002 | `toolchain_path` shims lookup (R3), vars from the resolved env first |
| FR-003 | No existence check anywhere in `toolchain_path`. Test A1 (missing folders) and Part B1 (installed later) |
| FR-005 | AI CLI env via `ai_cli_env_for` → `spawn_env_for`. MCP servers inherit it (Part B2) |
| FR-006 | Folders are put in front after `resolved_env_for` (environment-include) |
| FR-007 | `availability_in` / `ai_clis_available_in` use `spawn_env_for` (test A3) |
| FR-008 | The rule applies only with `offered.json` (R2, test A2) |
| FR-009 | `Settings::setup_commands`, Settings › Project (U1), `ProjectSetupSet` (C1, C3) |
| FR-010, FR-020 | `ops::create_worktree` → `start_setup`. `WorktreeInclude` and refresh never call it (R10) |
| FR-011 | `shell_line` plus `spawn_env_for(cwd)` in `setup.rs` |
| FR-012 | `StreamingChild` stdin null |
| FR-013 | Start after the create has answered. MCP `"setup"` field (C4) |
| FR-014 | Session start has no setup dependency (test A6: a session starts while Running) |
| FR-015 | `SetupChanged` → row `Tag` (U2) |
| FR-016 | `SetupOutputWatch`/`SetupOutput` → output dialog (U4); record and log kept until delete or re-run |
| FR-017 | `Failure` reason line. Client notification deduped by `RunId` (U5) |
| FR-018 | The run belongs to the daemon. Attach catch-up, re-watch on reconnect |
| FR-019 | Persisted record. `unwind` → `interrupt_all`. `Running` at load → Interrupted |
| FR-021 | `OutputBuffer` 10,000 lines plus `dropped`. Log compaction at 20,000 |
| FR-022 | `SetupRun` message, **Run setup** predicate, Default row included |
| FR-023 | `SetupStop`, `Terminator::terminate`, `stop_requested` → Stopped |
| FR-023a | `SetupRuns::start` under the lock → `Busy` |
| FR-024 | `ops::delete_worktree` calls `stop_and_wait` (bounded by `RUN_OVERRUN`) before `git worktree remove`, then forgets; a late `finish` for a forgotten entry is dropped. Confirmation line (U6) |
| FR-025 | The three user-guide pages above |

## Test strategy

| Requirement | Layer |
|---|---|
| FR-001–004, FR-006 order | core unit (A1) |
| FR-001, FR-005, FR-007, FR-008 applied | daemon unit around `with_sandbox_path` (A2, A3); real sandbox suite asserts `PATH` in the basic image; Part B1–B2 |
| FR-009 storage, protocol | core unit (A14, A15) |
| FR-009 Settings section | client unit (A17); visual pass U1 |
| FR-010–014, FR-020 | daemon integration (A6–A8, A10) |
| FR-012, FR-023 process tree | core unit on all three OSes (A5); daemon (A9) |
| FR-015, FR-022, FR-023 offering | client reducer (A16); geometry gate G1; visual pass U2/U3 |
| FR-016, FR-021 | core (A4 buffer); daemon watch (A6); geometry gate G2; visual pass U4 |
| FR-017, SC-004 | daemon reason line (A8); client dedupe (A16); Part B4 |
| FR-018, FR-019 | daemon (A11, A13); Part B7 |
| FR-023a | daemon (A9) |
| FR-024 | daemon (A12) |
| FR-025 | docs build and link check in CI; `scripts/check-user-guide-updated.sh` |
| SC-002, SC-005 | Part B3 (real mise). A6 asserts the create answers before the run ends |

The isolation and lifecycle gate: worktree create and delete with setup are covered by the daemon
integration tests (A6, A12), not by unit tests alone.

## Suggested milestones (for the tasks unit)

Each fits one session (task-size gate). They go in dependency order:

1. **M1 PATH**: `toolchain_path`, `with_sandbox_path`, tests A1–A3, the real sandbox assertion,
   and the `sandboxed-daemon.md` PATH and own-image text (FR-025 part). This ships alone and fixes
   Story 1.
2. **M2 Setup core**: the `setup` module, `StreamingChild`, settings and store additions, and
   protocol messages with the version bump (A4, A5, A14, A15).
3. **M3 Setup daemon**: `setup.rs`, the `ops` and `server` wiring, the MCP field, persistence,
   restart and unwind (A6–A13), and the `agent-tools.md` field.
4. **M4a Setup client state**: the reducer, `ProjectSetupSet` sync (A16), the Settings › Project
   section (A17, U1), and the `settings.md` doc.
5. **M4b Setup client UI**: the row tag, the menu, the output dialog, the notification, and the
   delete line (G1, G2); the `worktrees-and-sessions.md` doc; the visual pass (U2–U6).

## Complexity Tracking

None.
