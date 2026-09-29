# Implementation Plan: The session service exposes an MCP server to the AI sessions it runs

**Branch**: `feat/daemon-should-expose-mcp-server-for-agent` | **Date**: 2026-09-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/034-daemon-mcp-server/spec.md`

## Summary

The session service (`micold-daemon`) gains a second loopback HTTP listener, beside the hook
receiver, that speaks the small stateless subset of MCP Streamable HTTP (`initialize`, `ping`,
`tools/list`, `tools/call`) with a per-session bearer credential. Each Claude Code session is
spawned with `--mcp-config <file> --allowedTools mcp__micold`, each Copilot session with
`--additional-mcp-config @<file> --allow-tool micold`; the file lives in the service's own data
directory, owner-only, and no user or project configuration is touched. Pi has no MCP support and
stays unbound (FR-005).

Tool calls reuse the sidebar's own code paths: the worktree create, delete and rename bodies of
`route()` move into a shared `ops` module, and session operations call the existing `DaemonState`
methods, so validation, naming, provenance, gates and the `CatalogChanged` broadcast are the user
action's. Every policy rule of the spec (scope, self-refusals, Default-session refusals, the
destructive-operation confirmation and the cross-session option) is one pure, table-tested function
in `micold-core`. Destructive requests wait up to 60 s for an answer from a confirmation dialog that
every connected window shows.

Research (Phase 0) settled: the per-CLI launch flags (R1–R4), a hand-rolled server over the
existing HTTP code rather than the `rmcp` SDK (R5), a separate listener (R6), credentials and
owner-only files per platform, with FR-007 sharpened to what a loopback TCP transport can guarantee
(R7), the sandbox needing no change (R8), and the confirmation channel (R10).

## Technical Context

**Language/Version**: Rust, stable toolchain pinned by `rust-toolchain.toml`.

**Primary Dependencies**: no new crates. `tokio` (listener, oneshot, timeouts), `serde`/`serde_json`
(JSON-RPC and tool schemas), `uuid` (credentials), `tracing` (audit), `windows-sys` (file DACL, already
a Windows dependency of the daemon), `iced` (dialog and settings rows). External: `claude` ≥ 2.1
(`--mcp-config`, `--allowedTools`), `copilot` ≥ 1.0 (`--additional-mcp-config`, `--allow-tool`).

**Storage**: local files only. Per-session binding files `<data_dir>/mcp/<session>.json`
(owner-only, rewritten per spawn). Two new persisted settings in the existing settings file.
Credentials and pending confirmations are in memory only.

**Testing**: `mise run test-core` for the pure core (`crates/micold-core/tests/mcp_*.rs`);
`mise run test` / `mise run gate` for daemon integration tests (`crates/micold-daemon/tests/mcp_*.rs`,
real `DaemonState`, raw `TcpStream` requests as `hooks_receiver.rs` does, fake clients via
`state.register` as `mutation_semantics.rs` does) and client reducer tests
(`crates/micold-client/tests/`); `mise run image` + `mise run test-sandbox` for the container case;
quickstart §B for the real CLIs and the visual pass.

**Target Platform**: Linux, macOS, Windows desktop; host and sandboxed (027) placements.

**Project Type**: desktop application (three-crate Rust workspace: client, core, daemon).

**Performance Goals**: read-only tools answer in < 1 s for 50 worktrees + 50 sessions (SC-004) —
they read the in-memory catalog snapshot and worktree cache, no git subprocess. Agent-made changes
reach every window in < 2 s (SC-003) via the existing `broadcast_catalog`.

**Constraints**: loopback only; no user/project config writes; no body or input text in any log;
no new HTTP framework; no `match` on the concrete CLI outside `provider.rs` (feature 026 rule,
`crates/micold-client/tests/no_concrete_implementations.rs`).

**Scale/Scope**: 15 tools, 1 listener, 2 settings, 3 protocol bumps (M1 and M4 add a settings field
each; M3 adds the 3 confirmation messages), 1 new dialog, 1 new `DaemonState::stop_session`.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- [x] **I. Test-First (NON-NEGOTIABLE)**: PASS. Every rule is decision logic and lands in tested
  code first: `micold-core::mcp` (JSON-RPC codec, tool input validation, `policy::decide`,
  submission encoding, line clamping, binding-plan construction per `ToolServerSupport`) is
  table-tested; the daemon's endpoint, tools, spawn wiring and confirmation registry get
  integration tests in `crates/micold-daemon/tests/`. The only glue under the GUI exception is the
  dialog's and settings rows' rendering (`src/ui/`), validated by quickstart §B4/§B6 with the
  `visual-pass` skill; the dialog's reducer (`features/agent_confirm.rs`) is tested.
- [x] **II. Multi-Session Support**: PASS, with one recorded interpretation (Complexity Tracking). Credentials,
  binding files and audit attribution are per session; a credential acts as exactly one session;
  concurrent calls from many sessions are attributed separately and serialised per project by the
  existing worktree gate (SC-005). The cross-session tools (`read_session_output`,
  `send_session_input`) move data between sessions **only on an explicit agent request under a
  user-controlled option** (FR-016, default Auto). The spec's Assumption reads this as the user's own
  interaction through their agent — the same act as the user typing into that session's terminal —
  not as the implicit filesystem, in-memory or configuration leak Principle II forbids; no session's
  environment, files or configuration is shared or altered by another. Nothing is persisted across
  sessions.
- [x] **III. Worktree Integration**: PASS. Agent-made worktrees go through the app's own create path
  (location `.claude/worktrees/<name>`, provenance record, gate), so the app still owns their
  lifecycle; `create_session` places a session only in a worktree of the project or in Default. A
  Default session's agent cannot create, rename or delete a worktree (FR-015a), which is the
  principle's own rule applied to agent requests.
- [x] **IV. Local-First Storage (NON-NEGOTIABLE)**: PASS. Loopback only, in both placements; no
  remote service; everything works offline; nothing leaves the device.
- [x] **V. Rust + iced Stack**: PASS. Rust only; the dialog is iced. Invalid states are types:
  `LineCount` (1..=2000), `NonEmptyText`, `WorktreeRef::{Default, Named}`, `ToolServerSupport`,
  `ConfirmOutcome`, and `PolicyDecision` make "a Default caller deleting a worktree" or "an empty
  submission" unconstructible past validation.
- [x] **VI. Cross-Platform Parity**: PASS. TCP loopback, JSON and the CLIs' flags are identical on
  all three. The one platform difference — making the binding file owner-only — sits behind one
  function in `crates/micold-daemon/src/platform/{unix,windows}.rs` (`write_owner_only`), with a
  test per platform (`mcp_binding_file_mode.rs`, Unix mode bits; Windows DACL read back). CI builds
  and tests all three.
- [x] **VII. Documentation First-Class**: PASS. FR-019: a new user-guide page
  `docs/user-guide/agent-tools.md` (tools, bound CLIs, scope, confirmations, both options), entries
  in `settings.md` and `SUMMARY.md`, and a note in `sandboxed-daemon.md`. Each milestone updates the
  guide for what it ships.
- [x] **VIII. Reusable UI Component Foundation**: PASS. The confirmation dialog composes the shared
  Material dialog (`material::dialog`, `Button::filled/outlined`, `Surface::Dialog`) like
  `ui/confirm_session_remove.rs`, and registers as a floating surface per
  `docs/development/architecture.md`; the settings rows reuse the toggle and select components the
  Environment page already uses. No new widget.

**Post-Phase-1 re-evaluation**: unchanged, all eight PASS. The design added one security-relevant
sharpening (R7: FR-007/SC-009 worded for loopback TCP) that tightens rather than loosens the
requirement's testable form, and no complexity to track.

## Requirement map

| Requirement | Where |
|---|---|
| FR-001 | `mcp/server.rs` listener; R6, R8; contracts/binding.md §1 |
| FR-002, FR-003 | `ToolServerSupport` in `provider.rs`; `mcp/binding.rs`; R1–R4, R14; binding.md §4, §6 |
| FR-004 | setting `tool_server_enabled` (TM6); protocol-delta §1 |
| FR-005 | `BindingOutcome::Skipped` log line; binding.md §5; user guide |
| FR-006 | `Credentials` (TM2); binding.md §2 |
| FR-007 | loopback bind; `platform::write_owner_only`; R7 |
| FR-008 | `mcp/tools.rs`; contracts/mcp-tools.md |
| FR-009 | `ops.rs` extraction; `DaemonState::stop_session`; R9 |
| FR-010 | `Caller` scope resolution (TM4) |
| FR-011 | existing `broadcast_catalog` after each mutation |
| FR-012, FR-012a | `Framer::plain_tail`; `LineCount`, `NonEmptyText`; R11 |
| FR-013 | `OpError`/`ErrorCategory` (TM7); mcp-tools.md §Results |
| FR-014 | `mcp/confirm.rs` (TM5); protocol-delta §2; client `features/agent_confirm.rs` |
| FR-015, FR-015a | `policy::decide` |
| FR-016 | setting `cross_session_access`; `policy::decide`; protocol-delta §3 |
| FR-017 | `InputReadiness` seam + `create_session` prompt wait; R12 |
| FR-018 | audit line (TM7) |
| FR-019 | `docs/user-guide/agent-tools.md` + settings.md |

## Test strategy by layer

| Layer | What it proves |
|---|---|
| Core unit (`crates/micold-core/tests/mcp_*.rs`) | JSON-RPC parsing and responses; every tool's input validation; `policy::decide` as a table over (caller location, self/other, operation, option); binding plan per `ToolServerSupport`; collision detection over fixture config files; submission encoding; settings round-trip and defaults |
| Daemon integration (`crates/micold-daemon/tests/mcp_*.rs`) | endpoint auth/bounds/methods over real TCP; each tool against a real `DaemonState` and temp git repos; spawn wiring (argv contains the flags, config files byte-identical — SC-002); file modes; confirmation registry with fake clients; 10-way create race (SC-005); read latency with 50+50 (SC-004); audit lines (SC-010). Isolation & lifecycle gate: Default-session refusals and cross-project `not_found` are integration tests |
| Client (`crates/micold-client/tests/features_agent_confirm.rs`, settings tests) | the dialog's state: request shows, withdraw hides, answer sends `ConfirmationAnswer` once; settings drafts carry both new fields |
| Geometry gates (`crates/micold-client/tests/layout_snapshot.rs`, `layout_coverage_registry.rs` + `tests/support/covered_states.rs`) | the confirmation dialog and the Environment page with both new rows are registered covered states |
| Real runtime (`sandbox_real_mcp.rs`, off by default) | the binding works inside the container (SC-008 for 027) |
| Quickstart §B | real `claude` / `copilot` accept the binding and survive a ~55 s confirmation wait (R1–R3 probes); readiness per CLI (R12); dialog and settings rows' colour and weight (visual pass) |

## Project Structure

### Documentation (this feature)

```text
specs/034-daemon-mcp-server/
├── plan.md
├── spec.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── binding.md          # endpoint, auth, MCP methods, per-CLI launch wiring
│   ├── mcp-tools.md        # the 15 tools, inputs, outputs, policy, errors
│   └── protocol-delta.md   # client ↔ service wire changes per milestone
├── checklists/requirements.md
├── autopilot.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── mcp/                    # NEW — pure logic, no I/O
│   ├── mod.rs
│   ├── jsonrpc.rs          # request/response/notification types, error codes
│   ├── tools.rs            # tool catalog: names, descriptions, input schemas; typed Operation
│   ├── policy.rs           # decide(): FR-010/014/015/015a/016
│   ├── errors.rs           # ErrorCategory, OpError
│   ├── binding.rs          # BindingPlan from ToolServerSupport + url + credential; collision check
│   └── submission.rs       # bracketed-paste + CR encoding; output-settled readiness rule
├── provider.rs             # ToolServerSupport, InputReadiness + their AiCliProvider methods
├── settings.rs             # tool_server_enabled, cross_session_access
└── protocol/{messages,version}.rs   # DaemonSettings/SettingsSet fields; confirmation messages

crates/micold-daemon/src/
├── http.rs                 # NEW — bounded HTTP/1.1 head/body/respond/drain, extracted from hooks.rs
├── hooks.rs                # uses http.rs; hook behaviour unchanged
├── mcp/                    # NEW
│   ├── mod.rs
│   ├── server.rs           # listener, auth, JSON-RPC dispatch
│   ├── credentials.rs      # TM2
│   ├── tools.rs            # tool handlers → ops / DaemonState
│   └── confirm.rs          # TM5 registry
├── ops.rs                  # NEW — worktree create/delete/rename bodies moved out of route()
├── framer.rs               # plain_tail()
├── state.rs                # tool server handle; binding at spawn; primary-process write/read;
│                           #   stop_session(); ready-for-input marks
├── hooks.rs / event_log.rs # SessionStart / Pi session_start also mark ready (R12)
├── server.rs               # binds the listener; route() calls ops.rs; ConfirmationAnswer arm
├── catalog.rs              # the two settings
└── platform/{mod,unix,windows}.rs   # write_owner_only()

crates/micold-client/src/
├── features/agent_confirm.rs   # NEW — pending prompts state + reducer
├── ui/confirm_agent_request.rs # NEW — dialog view (shared Material dialog)
├── overlay/registry.rs         # one registration line
├── features/settings.rs        # two new drafts/messages
├── ui/settings/environment.rs  # two rows
└── shell/daemon_sync.rs        # ConfirmationRequested / Withdrawn arms

docs/user-guide/{agent-tools.md (NEW), settings.md, sandboxed-daemon.md}, docs/SUMMARY.md
```

**Structure Decision**: the existing three-crate workspace. Everything with a decision in it goes to
`micold-core::mcp` so it is testable without a daemon (`mise run test-core`); the daemon holds I/O
(listener, files, PTYs, the registry); the client holds only the dialog and two settings rows.

## Milestone outline (cut in tasks.md)

1. **M1 — US1**: listener, credentials, binding for Claude and Copilot, the read-only tools, the
   FR-004 toggle, user guide page. Deliverable: a new Claude Code session can call `whoami`,
   `list_worktrees`, `list_sessions` with no setup.
2. **M2 — US2**: `ops.rs` extraction, `create_worktree`, `create_session` with prompt.
3. **M3 — US3**: policy for destructive operations, confirmation registry, wire messages, dialog,
   lifecycle tools.
4. **M4 — US4**: `read_session_output`, `send_session_input`, the FR-016 option.
5. **M5 — Polish**: audit-log conformance, sandbox real-runtime test, SC-004 timing, docs pass.

## Risks

- **A CLI flag behaves differently than documented** (R1–R3 probes). Mitigation: quickstart §B1/§B2
  run before M1 merges; a failing CLI falls back to `Unsupported` with the observed reason (FR-005),
  which the spec already allows for Copilot; for Claude it would be an escalation (category 6).
- **Protocol number collisions** with concurrent features: take the next free number at rebase time.
- **`route()` extraction regressions**: the existing mutation test suites run unchanged against the
  extracted code before any tool uses it.

## Complexity Tracking

No constitution violations to justify.

**Recorded interpretation (Principle II)**: explicit, user-gated cross-session I/O —
`read_session_output` and `send_session_input`, each an addressed request, scoped to the caller's
project, governed by the FR-016 option and audited — is not the implicit filesystem, in-memory or
configuration leak Principle II forbids. It uses only the target's own input stream, the channel the
user already types into, and shares or alters no session's environment, files or configuration.
Confirmed by the plan review (round 1). The isolation & lifecycle gate still applies: Off,
cross-project `not_found` and per-request option changes are integration tests (`mcp_cross_session.rs`).
