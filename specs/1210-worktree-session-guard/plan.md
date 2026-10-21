# Implementation Plan: Worktree session guard

**Branch**: `feat/1210_guard-stop-sessions-from-working-inside-a-worktree` | **Date**: 2026-10-21 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1210-worktree-session-guard/spec.md`

## Summary

Micold enforces, for every launched Claude Code session, that it only touches its own worktree. Two mechanisms, both delivered through the per-session `--settings` file the daemon already writes (`crates/micold-daemon/src/hooks.rs::settings_json`): (1) a pure decision function in `micold-core` (new module `worktree_guard`) judges a `PreToolUse` payload against the session's own binding and the project's worktree list; the existing loopback hook receiver answers `PreToolUse` with a `permissionDecision: "deny"` body when it says deny; (2) static `permissions.deny` rules in the same settings file block `EnterWorktree` and every `git worktree add` form. One app-wide opt-out setting (beside `tool_server_enabled`) removes both at launch. The `create_worktree` MCP result gains the hand-off text. Decisions: [research.md](research.md).

## Technical Context

**Language/Version**: Rust stable (mise-managed)

**Primary Dependencies**: existing only: `serde_json`, `micold-core::worktree` (`parse_worktrees`), `micold-core::git`, `dunce`-style path normalisation if already a dependency (else std canonicalize with a pure lexical fallback); no new crates

**Storage**: one boolean in the daemon's existing settings file (`DaemonSettings`), local only

**Testing**: `cargo test -p micold-core` (decision logic, path forms, shell parsing, settings JSON merge shape); `micold-daemon` tests for the hook receiver response, settings emission and opt-out; MCP result text test; no GUI test

**Target Platform**: Linux, macOS, Windows (pure string/path logic; Windows and case-insensitive rules by injected normaliser)

**Project Type**: desktop app, three crates (core, daemon, client)

**Performance Goals**: under 100 ms per decision (SC-005); worktree list cached per session and refreshed on catalog change, never `git` spawned per call

**Constraints**: user configuration never modified (`--settings` only); fail open on unknown state (FR-009); additive to project rules (FR-013)

**Scale/Scope**: tens of worktrees per project; one decision per tool call

## Constitution Check

- [x] **I. Test-First**: every task pairs a failing `micold-core` / daemon test first; the thin daemon wiring is covered by receiver tests.
- [x] **II. Multi-Session**: the guard is derived per session from its own id, binding and the project's current list; no shared mutable state.
- [x] **III. Worktree Integration**: sessions stay bound to a worktree or Default; `create_worktree` then `create_session` is the only route; the Default limits are unchanged.
- [x] **IV. Local-First**: all decisions are local; no network.
- [x] **V. Rust + iced**: Rust only; `GuardVerdict`, `SessionBinding` enums make unknown/own/other states explicit; no GUI change beyond the settings toggle.
- [x] **VI. Cross-Platform**: one pure decision core with an injected path normaliser; tests include Windows drive-letter and case-folded layouts on every OS (SC-006).
- [x] **VII. Documentation**: `docs/user-guide/settings.md`, `agent-tools.md`, `worktrees-and-sessions.md` updated in the same change.
- [x] **VIII. UI Components**: the setting toggle reuses the existing settings toggle row component; no new widget.

Post-design re-check: unchanged, PASS. Complexity Tracking: none.

## Project Structure

### Documentation (this feature)

```text
specs/1210-worktree-session-guard/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── pretooluse-guard.md
│   └── settings-and-messages.md
└── tasks.md             # tasks unit
```

### Source Code

```text
crates/micold-core/src/worktree_guard/     # NEW
├── mod.rs       # decide(), GuardInput, GuardVerdict, deny message
├── paths.rs     # lexical + injected-normaliser path containment (FR-014)
└── shell.rs     # best-effort Bash analysis (FR-004)
crates/micold-core/src/protocol/messages.rs  # worktree_guard_enabled on DaemonSettings + request (version bump)
crates/micold-daemon/src/hooks.rs          # settings_json adds permissions.deny; PreToolUse deny response
crates/micold-daemon/src/catalog.rs        # worktree_guard_enabled setting (beside tool_server_enabled)
crates/micold-daemon/src/state.rs          # per-session guard context at launch; frozen opt-out state
crates/micold-daemon/src/mcp/tools.rs      # create_worktree result text (FR-010)
crates/micold-client/src/features/settings/ # toggle row
docs/user-guide/{settings,agent-tools,worktrees-and-sessions}.md
```

**Structure Decision**: logic in `micold-core` (testable, platform-neutral); I/O and wiring in `micold-daemon`; client only adds the toggle.

## Requirement map

| FR | Where |
|---|---|
| FR-001, FR-008 | `state.rs` launch path adds guard to `--settings` unless setting off; `catalog.rs` setting; client toggle |
| FR-002, FR-011 | `worktree_guard::decide` over Edit/Write/MultiEdit/NotebookEdit `file_path` and Bash `cwd`/command |
| FR-003, FR-010 | `worktree_guard::DENY_MESSAGE`; `mcp/tools.rs` result text |
| FR-004 | `worktree_guard::shell` |
| FR-005, FR-006 | `permissions.deny` in `hooks.rs::settings_json`; `mcp__micold__create_worktree` untouched |
| FR-007 | allowlist in `decide` (see research R5) |
| FR-009 | `GuardVerdict::AllowUnknown(reason)` logged by receiver |
| FR-012, FR-014 | tests in `worktree_guard/*` |
| FR-013 | deny rules and hooks are additive in `--settings`, which Claude merges with project settings (test: emitted JSON has no `allow`/override keys) |

## Test strategy

| Requirement | Layer |
|---|---|
| FR-002, 004, 007, 011, 014, SC-001/3/6 | core unit (`worktree_guard`) |
| FR-005, 013, SC-002/7 | daemon unit: emitted `--settings` JSON; manual quickstart against real `claude` |
| FR-008, SC-004 | daemon unit (launch with setting off emits no guard) |
| FR-010 | daemon unit on result text |
| Toggle | client unit; no geometry/visual change (existing row) |
