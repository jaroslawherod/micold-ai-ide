# Implementation Plan: Codex CLI and OpenCode as session providers

**Branch**: `claude/project-thread-wysm57` | **Date**: 2026-10-09 | **Spec**: [spec.md](./spec.md)

## Summary

Add `AiCli::Codex` and `AiCli::OpenCode` behind the existing `AiCliProvider` seam
(`crates/micold-core/src/provider.rs`), as feature 029 did for Pi. Two seam gaps (research R1, R5)
need small trait additions (required methods, no defaults): both CLIs **mint their own conversation ids** (so a session
binds to the id after start), and sandbox **sign-in sharing is not per provider** today. Activity is
`None` (Unknown) and tool-server binding is `Unsupported` for v1; the CLI assumptions and their
fallbacks are in [research.md](./research.md).

## Technical Context

**Language**: Rust (stable, `rust-toolchain.toml`). One non-Rust edit: `packaging/sandbox/Containerfile`.
**Dependencies**: no new crates (`serde_json`, `uuid`, `directories` exist). External: `codex`, `opencode`.
**Storage**: local only. Reads Codex `~/.codex/sessions`; reads OpenCode through its own CLI
(bounded, best-effort). Writes: app-owned binding/archive files under `<config_dir>/micold-bindings/`,
outside either CLI's session store.
**Testing**: `mise run test-core` (seam, fixtures), `mise run test` (daemon/client), `mise run image`
+ `mise run test-sandbox` (`sandbox_real_ai_cli.rs` iterates `AiCli::ALL`). Stand-in `codex`/`opencode`
executables on `PATH` for daemon tests, as `session_start.rs` does.
**Platforms**: Linux/macOS/Windows; `cfg` only if the probe finds a divergent store path.
**Constraints**: no concrete provider named outside `provider.rs` (`micold-client/tests/no_concrete_implementations.rs`);
no second enumeration of CLIs (use `AiCli::ALL`); existing providers' tests unchanged (SC-007).

## Constitution Check

| Principle | Result |
|---|---|
| I Test-First | PASS: each seam method and provider lands behind a failing fixture test (`codex_provider.rs`, `opencode_provider.rs` mirroring `pi_provider.rs`); image obligation red once variants exist. |
| II Multi-Session | PASS: bind-only-if-exactly-one-candidate rule; ambiguity ⇒ unbound ⇒ fresh, never another session's conversation (R1). |
| III Worktree Integration | PASS: conversations matched by session cwd; no worktree logic changes. |
| IV Local-First (NON-NEGOTIABLE) | PASS: nothing remote added; OpenCode read is a local process; telemetry switches recorded per R8. |
| V Rust + iced | PASS: pure Rust, no new crate (SQLite reader rejected, R2). |
| VI Cross-Platform | PASS with probe: executable names via existing PATH resolver; store paths home-relative; `cfg` only if probe shows otherwise. |
| VII Documentation | PASS: user-guide updates (FR-014) are tasks, plus README/provider lists. |
| VIII UI Components | PASS: provider list/label reuse existing components; only data (variants) added. Visual pass: provider chooser lists 5 entries, unavailable reasons (quickstart §B). |

## Requirement → design map

| FR | Where |
|---|---|
| FR-001 | `session.rs` `AiCli` + `ALL`, `provider.rs` `tool_name`/`provider()`; Settings, new-session and MCP `ai_cli` all derive from `AiCli::ALL`/`tool_name` (verify in tasks by grep; fix any hand-written list) |
| FR-002 | `store.rs` `StoredAiCli` arms (+ round-trip test with a pre-feature file) |
| FR-003/004 | existing `is_available`/`resolves_on_path`, `ops.rs::cli_unavailable`, `server.rs` `AiCliAvailabilityRequest`; new providers only implement `is_available` (tests: stand-in on/off PATH, MCP refusal) |
| FR-005 | `command()`, `launch_args` fresh = none |
| FR-006/007 | R1 identity + `list_conversations`; `launch_args_bound`; binding files (R1) read by the provider's own methods, so the prune sweeps (`state.rs`) and client `persist.rs` need no change; bind poll in `daemon/state.rs` spawn path; Codex R3 and OpenCode R2 readers, best-effort |
| FR-008 | `read_title` (Codex prefix parse; OpenCode export) |
| other seam methods | `name_in_terminal_title` ⇒ `None`; `read_label` ⇒ `None` (title carries the name); `store_dirs` and `last_activity` inherit the existing trait defaults; `recorded_session_ids` ⇒ empty for Minted providers (discovery of their conversations is 582's; a bound session is found through its binding file). One test each in the provider test files |
| `launch_args_in` caller | `micold-core/src/terminal.rs:54` `launch_args` resolves `provider.config_dir()` itself and calls `launch_args_in`; `LaunchSpec` is unchanged; a `None` config dir ⇒ fresh start. `terminal_backend.rs` tests updated |
| FR-009 | `activity_source` = `ActivitySource::None` (daemon test: badge `Unknown` through output/silence) |
| FR-010 | `mark_archived`/`is_archived` app-owned markers (R1/R3) |
| FR-011/012 | `tool_server_support` = `Unsupported`, `input_readiness` = `OutputSettled`, `folder_trust` = `CodexProjects`/`NeverAsks` |
| FR-013 | R5 `sandbox_auth_file`, `CredentialLayout.ai_cli_auth: Vec`, `shell/sandbox.rs`:105/:476 and their tests; Containerfile (R7) |
| FR-014 | `docs/user-guide/worktrees-and-sessions.md`, `settings.md`, `agent-tools.md`, `sandboxed-daemon.md`, `docs/install.md`, README: new sections per provider |
| FR-015 | existing providers return their old values from the new required methods; per-provider behaviour only inside `provider.rs` |
| FR-016 | `protocol/version.rs` 38→39 + `hashing.rs`/`schema_hash.rs` pins in one edit; test: old-version handshake refused |

## Test strategy by layer

- **core unit** (`mise run test-core`): provider trait surface per provider against fixture stores
  (rollout tree, stub `opencode` script printing JSON); bind rule table (0/1/2 candidates, already
  bound, wrong cwd); store round-trip with old file; protocol hash/version.
- **daemon**: stand-in executables — start, restart resumes bound id, unavailable refusal via app
  and MCP, `Unknown` activity, two concurrent same-cwd sessions never cross-resume.
- **client**: provider chooser/Settings list five entries with unavailable reasons; geometry gates
  unchanged; no-concrete-implementations test stays green.
- **sandbox**: `mise run test-sandbox` version commands in image; layout test for
  `sandbox_auth_file` mounting (present/absent).
- **quickstart §B visual pass**: chooser rows and unavailable reason text.

## Project Structure

```
crates/micold-core/src/{provider.rs, session.rs, store.rs, sandbox/mod.rs, protocol/{version,hashing}.rs}
crates/micold-core/tests/{codex_provider.rs, opencode_provider.rs, ai_cli_provider.rs(extend)}
crates/micold-daemon/src/{state.rs (bind step), ops.rs}  crates/micold-daemon/tests/
crates/micold-client/src/shell/sandbox.rs  packaging/sandbox/Containerfile  docs/user-guide/*
```
All paths exist except the two new test files (marked new).

## Complexity Tracking

| Addition | Why | Simpler alternative rejected |
|---|---|---|
| `ConversationIdentity::Minted` + binding files + bind poll; five required trait methods (no defaults, trait rule FR-021; `FakeAiCliProvider` updated) | Codex/OpenCode cannot take an app-chosen id (V5) | `--last`/`--continue` violates FR-006 |
| `sandbox_auth_file` + `ai_cli_auth: Vec` | sign-in sharing was one claude-only field (R5) | per-CLI conditional in sandbox code violates FR-015 |
| OpenCode read via its own CLI | store is a database (V9) | `rusqlite` dependency (R2) |
