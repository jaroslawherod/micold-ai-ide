---

description: "Task list for #488 Codex CLI and OpenCode as session providers"
---

# Tasks: Codex CLI and OpenCode as Session Providers

**Input**: `specs/488-codex-opencode-providers/` (spec.md, plan.md, research.md, data-model.md, contracts/)
**Tests**: mandatory and first (Constitution I): in every milestone the test tasks come before the code they cover and must be seen failing.
**Docs**: each story's user-guide task sits in that story's milestone (Constitution VII, `scripts/check-user-guide-updated.sh`).
**Cross-platform**: core logic is platform-agnostic; `cfg` only where T001 finds a divergent store path.
**Format**: `[ID] [P?] [Story] Description` — `[P]` = different files, no dependency.

## Phase 1: Setup

- [ ] T001 Probe the real CLIs first. With `codex` and `opencode` installed (npm `@openai/codex`, `opencode-ai`; no install possible ⇒ record "not probed" and keep every fallback), settle V5 (OpenCode id flag), V8 (Codex first-turn line shape), V10 (OpenCode `session list --format json` / `export` field names), V13 (activity signal), V14 (per-launch MCP binding), V15 (folder-trust prompt), R8 (update-check/telemetry switches), the store paths per platform and the current versions to pin. Record results in `specs/488-codex-opencode-providers/contracts/codex-cli.md`, `contracts/opencode-cli.md` and the V-table of `research.md`; a changed assumption updates `plan.md` and the later tasks.
- [ ] T002 Audit for hand-written provider lists or `match` arms outside `provider.rs` (Settings default, new-session chooser, MCP `ai_cli` parameter/schema, store, labels): `grep -rn "Pi\b\|\"pi\"\|AiCli::" crates/*/src`. Record each place and whether it derives from `AiCli::ALL`/`tool_name` in `research.md` (R9). Files: `specs/488-codex-opencode-providers/research.md`.

## Phase 2: User Story 1 — Start a Codex or OpenCode session (P1) 🎯 MVP

- [ ] T003 [P] [US1] Failing tests for the fresh-start provider surface of both: command, `tool_name`, `is_available` with a stand-in on/off `PATH`, fresh `launch_args` (none), `name_in_terminal_title`/`read_label` ⇒ `None`, `recorded_session_ids` empty, `tool_server_support` `Unsupported`, `input_readiness` `OutputSettled`, `folder_trust` (`CodexProjects`/`NeverAsks`), `activity_source` `None`, `launch_env` (empty unless T001 finds a switch); availability with `.cmd`/`.exe` stand-ins via the PATH resolver where the platform allows. Files: `crates/micold-core/tests/codex_provider.rs` (new), `crates/micold-core/tests/opencode_provider.rs` (new), mirroring `pi_provider.rs`.
- [ ] T004 [P] [US1] Failing tests: `StoredAiCli` round-trip of Codex/OpenCode and load of a pre-feature store file unchanged (FR-002); `PROTOCOL_VERSION` is 39 and an old-version handshake is refused (FR-016). Files: `crates/micold-core/src/store.rs` tests, `crates/micold-core/tests/` (the protocol/handshake tests that pin 38).
- [ ] T005 [US1] Failing daemon tests with stand-in `codex`/`opencode` on `PATH` (as `session_start.rs`): start runs the stand-in in the worktree and the row is labelled `codex`/`opencode`; restart keeps the provider; Settings default Codex/OpenCode applies; MCP `create_session` with `ai_cli` `codex`/`opencode` starts it. Files: `crates/micold-daemon/tests/` (new file `codex_opencode_start.rs` beside `session_start.rs`).
- [ ] T006 [US1] Add `AiCli::Codex`/`OpenCode` to the enum and `ALL: [AiCli; 5]`, and `StoredAiCli` arms. Files: `crates/micold-core/src/session.rs`, `crates/micold-core/src/store.rs`.
- [ ] T007 [US1] Add `CodexProvider` and `OpenCodeProvider` (fresh start only; resume and binding arrive in M3/M4) with `provider()`/`tool_name` arms, plus `FolderTrust::CodexProjects` reading `config.toml` `trust_level` (unrecorded ⇒ the existing `AsksTrust` refusal path in `ops.rs`). Files: `crates/micold-core/src/provider.rs`, `crates/micold-daemon/src/ops.rs`.
- [ ] T008 [US1] Bump `PROTOCOL_VERSION` 38 → 39 with `protocol/hashing.rs` and `schema_hash.rs` pins in the same edit. Files: `crates/micold-core/src/protocol/version.rs`, `hashing.rs`, `schema_hash.rs`.
- [ ] T009 [US1] Fix every hand-written list or arm T002 found so chooser, Settings and MCP all come from `AiCli::ALL`/`tool_name`; add a client test that the chooser and Settings list five entries; `no_concrete_implementations.rs` stays green. Files: as T002 records (`crates/micold-client/src/`, `crates/micold-daemon/src/`), `crates/micold-client/tests/`.
- [ ] T010 [US1] Set any update-check/telemetry environment switch T001 found in each provider's `launch_env()` (none found ⇒ none set, state so in the contract file). Files: `crates/micold-core/src/provider.rs`.
- [ ] T011 [US1] User guide: list Codex and OpenCode wherever providers are listed and add a section per provider on detection and start. Files: `docs/user-guide/worktrees-and-sessions.md`, `docs/user-guide/settings.md`, `docs/user-guide/agent-tools.md`, `docs/install.md`, `README.md`.

## Phase 3: User Story 2 — See why a provider cannot be used (P1)

- [ ] T012 [US2] Characterization tests (they pass on existing behaviour; no production code follows from them unless one fails, which is then the red): both unavailable with a reason naming the command when off `PATH`; start refused before any terminal exists, from the app and from MCP; availability re-evaluated on the next listing after install; a session whose provider was removed is listed with its remembered provider, unavailable. Files: `crates/micold-daemon/tests/` (extend T005's file), `crates/micold-core/tests/codex_provider.rs`, `opencode_provider.rs`.
- [ ] T013 [US2] Close any gap T012 exposes in availability, refusal or listing (`is_available`, `ops.rs::cli_unavailable`, `server.rs` `AiCliAvailabilityRequest`); no gap ⇒ record "no code change" in the ledger. Files: `crates/micold-daemon/src/ops.rs`, `crates/micold-daemon/src/server.rs`.
- [ ] T014 [US2] Client test that the chooser and Settings show an unavailable provider with its reason, then run the `visual-pass` skill for quickstart §B (stand-in `codex` on `PATH`, no `opencode`) and save the evidence. Files: `crates/micold-client/tests/`, `specs/488-codex-opencode-providers/visual-pass.md`.
- [ ] T015 [US2] User guide: what an unavailable provider shows, and that installing the CLI makes it available on the next listing. Files: `docs/user-guide/worktrees-and-sessions.md`.

## Phase 4: User Story 3 — Resume and name a session (P2), Codex

- [ ] T016 [US3] Failing tests for the seam additions on the existing providers: claude/copilot/pi `identity()` `AppAssigned`, `launch_args_in` equals `launch_args`, `new_conversations` empty, `bind` returns `Ok(())` without writing, and `FakeAiCliProvider` implements all (SC-007: old tests unchanged). Files: `crates/micold-core/tests/ai_cli_provider.rs`.
- [ ] T017 [US3] Add the required trait methods `identity()` (`ConversationIdentity::{AppAssigned,Minted}`), `launch_args_in(config_dir, session_id, mode)`, `new_conversations(config_dir, cwd, since)`, `bind(config_dir, session_id, &ConversationRef)`; implement for claude/copilot/pi/`FakeAiCliProvider`; `terminal.rs` `launch_args` resolves `config_dir()` and calls `launch_args_in`; update `terminal_backend.rs` tests. Files: `crates/micold-core/src/provider.rs`, `crates/micold-core/src/terminal.rs`, `crates/micold-core/tests/terminal_backend.rs`.
- [ ] T018 [US3] Failing Codex tests on a fixture rollout tree: candidates need matching `cwd`, mtime ≥ spawn − 2 s, id not bound elsewhere; bind-only-if-exactly-one table (0/1/2 candidates, already bound, wrong cwd); bound ⇒ `codex resume <id>`, unbound ⇒ fresh; `has_recorded_conversation`; label from the first turn in a ≤64 KiB prefix (shape per T001); `mark_archived`/`is_archived`; missing/unreadable store ⇒ nothing. Files: `crates/micold-core/tests/codex_provider.rs`.
- [ ] T019 [US3] Implement Codex `identity` `Minted`, store reader (R3), the shared `micold-bindings` binding/archive helpers, `launch_args_in`, `new_conversations`, `bind`, `read_title`, archive markers. Files: `crates/micold-core/src/provider.rs`.
- [ ] T020 [US3] Failing daemon tests with a stand-in `codex` that writes a rollout file: the daemon binds after spawn; restart resumes the bound id; two concurrent same-cwd sessions never cross-resume; close records the archive marker; prune sweeps and `shell/persist.rs` need no change. Files: `crates/micold-daemon/tests/` (new file).
- [ ] T021 [US3] Daemon bind step in the spawn path: poll `new_conversations` every 2 s for ≤60 s, then give up unbound (R1). Files: `crates/micold-daemon/src/state.rs`.
- [ ] T022 [US3] User guide: Codex restart behaviour, naming, and what happens when no conversation is recorded. Files: `docs/user-guide/worktrees-and-sessions.md`.

## Phase 5: User Story 3 — Resume and name a session (P2), OpenCode

- [ ] T023 [US3] Failing OpenCode tests with a stub `opencode` script printing JSON (fields per T001): list → candidates by `directory`, created after spawn, not bound elsewhere; `opencode --session <id>` on resume; label from export; 2 s timeout and unavailable CLI ⇒ nothing read; bad JSON ⇒ unbound. Files: `crates/micold-core/tests/opencode_provider.rs`.
- [ ] T024 [US3] Implement OpenCode `Minted` identity, bounded CLI reader (R2), binding under `~/.local/share/opencode/micold-bindings/`, `launch_args_in`, `new_conversations`, `bind`, `read_title`, archive markers. Files: `crates/micold-core/src/provider.rs`.
- [ ] T025 [US3] Daemon tests with the stub: start, bind, restart resumes `--session <id>`; two concurrent same-cwd sessions never cross-resume; close records the archive marker (FR-010, Principle II). Files: `crates/micold-daemon/tests/` (T020's file).
- [ ] T026 [US3] User guide: OpenCode restart behaviour and naming. Files: `docs/user-guide/worktrees-and-sessions.md`.

## Phase 6: User Story 4 — Activity that is honest (P2)

- [ ] T027 [US4] Characterization daemon tests (pass once T007 sets `ActivitySource::None`; they pin FR-009 against later change): Codex and OpenCode badges stay `Unknown` through output and silence; a provider with an activity source still follows busy/idle (AS2, existing providers). Files: `crates/micold-daemon/tests/`.
- [ ] T028 [US4] Characterization tests (red only where T029 needs code): `Unsupported` tool-server support is logged with the reason and the session starts; `create_session` with `prompt` is refused as `AsksTrust` for Codex in an untrusted dir and accepted for a trusted dir and for OpenCode. Files: `crates/micold-daemon/tests/`.
- [ ] T029 [US4] Close any gap T027/T028 expose (logging, readiness wiring); no gap ⇒ "no code change" in the ledger. Files: `crates/micold-daemon/src/ops.rs`, `crates/micold-daemon/src/state.rs`.
- [ ] T030 [US4] User guide: per provider, the activity badge (`Unknown`), tool-server support, first-prompt/folder-trust behaviour. Files: `docs/user-guide/worktrees-and-sessions.md`, `docs/user-guide/agent-tools.md`.

## Phase 7: User Story 5 — Use them in the sandbox (P3)

- [ ] T031 [US5] Failing tests: `CredentialLayout.ai_cli_auth` is a `Vec` filled from `AiCli::ALL` (claude first); `sandbox_auth_file` per provider (claude unchanged, copilot/pi `None`, Codex `$CODEX_HOME`-under-home else `~/.codex/auth.json`, OpenCode `~/.local/share/opencode/auth.json`); per-platform path cases (home-relative, relocated `$CODEX_HOME`) as pure path tests; present files mount read-only at the same home-relative path, absent ones are pruned and the sandbox still starts. Files: `crates/micold-core/tests/sandbox_credentials.rs`, `crates/micold-client/src/shell/sandbox.rs` tests.
- [ ] T032 [US5] Add required `sandbox_auth_file(home) -> Option<PathBuf>`; `ai_cli_auth: Vec<PathBuf>`; `path_for` returns a slice; `shell/sandbox.rs` prunes (:105) and mounts (:476) each; update `sandbox_real_ai_cli_sessions.rs:89` and the other readers mechanically. Files: `crates/micold-core/src/provider.rs`, `crates/micold-core/src/sandbox/mod.rs`, `crates/micold-client/src/shell/sandbox.rs`, `crates/micold-daemon/tests/sandbox_real_ai_cli_sessions.rs`.
- [ ] T033 [US5] Add `@openai/codex` and `opencode-ai` to the pinned `npm install -g` layer with `CODEX_CLI_VERSION`/`OPENCODE_CLI_VERSION` ARGs set to the versions T001 recorded (T001 not probed ⇒ resolve them with `npm view <pkg> version` before this task, else stop and escalate: no unpinned install); `sandbox_real_ai_cli.rs` iterates `AiCli::ALL`. Files: `packaging/sandbox/Containerfile`.
- [ ] T034 [US5] User guide: sandbox sign-in per provider (read-only single file, no refresh persisted, relocated `$CODEX_HOME` and keyring-only hosts not shared). Files: `docs/user-guide/sandboxed-daemon.md`.

## Polish (left to the close unit — changes no code)

- [ ] T035 Grep the user guide for each provider and topic (SC-006, US6); run quickstart Parts A and C and record results; `speckit-converge`.

## Dependencies

T001 → T007, T010, T018, T023, T033. T002 → T009. Within a milestone: tests → implementation → docs. M3 needs M1's variants; M4 needs M3's seam; M6 needs M4's `Provider` impls to add `sandbox_auth_file` to all five.

## Implementation Strategy

MVP = M1 (start works, provider remembered). Then unavailability, Codex resume/naming, OpenCode resume/naming, honest activity, sandbox.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Start a Codex or OpenCode session 🎯 MVP

- **Tasks**: T001–T011
- **Deliverable**: Codex and OpenCode appear in the chooser, Settings default and MCP `ai_cli`, start in the worktree, and are remembered across restart; wire protocol is 39.
- **Satisfies**: US1 acceptance scenarios 1–5; FR-001, FR-002, FR-005, FR-014 (per-story guide tasks), FR-015, FR-016
- **Verify**: `mise run test-core` and `cargo test -p micold-daemon --test codex_opencode_start`
- **Depends on**: —
- **Tier**: full

### M2 — Unavailable providers are explained

- **Tasks**: T012–T015
- **Deliverable**: a provider whose command is missing is listed unavailable with the reason and cannot be started from the app or MCP; becomes available on the next listing.
- **Satisfies**: US2 acceptance scenarios 1–3; FR-003, FR-004
- **Verify**: T012 tests; quickstart §B recorded in `visual-pass.md`
- **Depends on**: M1
- **Tier**: light

### M3 — Codex resumes and names its session

- **Tasks**: T016–T022
- **Deliverable**: a restarted Codex session resumes its own conversation (never another session's) and is labelled from its first turn; seam gains minted-identity methods with other providers unchanged.
- **Satisfies**: US3 acceptance scenarios 1–4 for Codex; FR-006, FR-007, FR-008, FR-010
- **Verify**: `cargo test -p micold-core --test codex_provider` and the T020 daemon test
- **Depends on**: M1
- **Tier**: full

### M4 — OpenCode resumes and names its session

- **Tasks**: T023–T026
- **Deliverable**: the same for OpenCode, read through its own CLI.
- **Satisfies**: US3 acceptance scenarios 1–4 for OpenCode; FR-006, FR-007, FR-008, FR-010
- **Verify**: `cargo test -p micold-core --test opencode_provider` and the T025 daemon test
- **Depends on**: M3
- **Tier**: full

### M5 — Honest activity, tool-server and first-prompt behaviour

- **Tasks**: T027–T030
- **Deliverable**: Codex/OpenCode badges read `Unknown` through output and silence; unsupported tool-server binding is logged; prompt injection never types into a trust question.
- **Satisfies**: US4 acceptance scenarios 1–2; FR-009, FR-011, FR-012
- **Verify**: T027/T028 tests
- **Depends on**: M1
- **Tier**: light

### M6 — Sandbox image and sign-in sharing

- **Tasks**: T031–T034
- **Deliverable**: the image carries both pinned CLIs and each CLI's sign-in file is shared read-only into the sandbox.
- **Satisfies**: US5 acceptance scenarios 1–3; FR-013
- **Verify**: `mise run image && mise run test-sandbox`; T031 tests
- **Depends on**: M4
- **Tier**: full
