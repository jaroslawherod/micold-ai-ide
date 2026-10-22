# Tasks: Worktree session guard

**Input**: `specs/1210-worktree-session-guard/` (spec.md, plan.md, research.md, data-model.md, contracts/, quickstart.md)

**Tests**: Constitution I: every behaviour has a failing test written first, in the same milestone as its code.

**Documentation**: Constitution VII: each milestone updates its user-guide page in the same PR.

**Cross-platform**: Constitution VI: the core logic is pure; path tests inject Windows, macOS and Linux layouts through `PathFacts` and run on every OS.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [X] T001 [US4] Confirm the autopilot prefetch worktree name (plan flag): `grep -rniI prefetch scripts .claude docs CLAUDE.md` finds nothing, so the `autopilot-prefetch` prefix of research R5 stays a new core constant that no repo convention backs. Recorded in `research.md` R5 (done in the tasks unit).
- [X] T002 [US1] Spike (plan flag, quickstart B step 1, run first): with a scratch HTTP hook server that answers `PreToolUse` with the deny body of `contracts/pretooluse-guard.md`, launch the real `claude` CLI (`--settings` file with an `http` PreToolUse hook), ask it to edit a file, and record in `research.md` R1 *Evidence* whether the agent saw the denial text and the edit did not happen; also check that a `permissions.deny` entry `Bash(git * worktree add*)` blocks `git -C . worktree add ../x`. Scratch files stay in the session scratchpad. If the CLI does not honour the JSON deny body, stop and escalate: R1 and the plan's enforcement channel change. Also check that a project `.claude/settings.json` with its own deny/allow rules and hooks still applies next to the `--settings` file (FR-013, SC-007). Fallbacks to put in the escalation: a command hook per R1's rejected option, or sandbox filesystem rules. If no `claude` CLI is available, record that, and leave quickstart B steps 1 and 2 as the only proof.

## Phase 2: Foundational (core module, no wiring)

- [X] T003 [P] [US1] Write failing tests for `crates/micold-core/src/worktree_guard/paths.rs`: lexical `.`/`..`, symlinked ancestor, case-fold on a case-insensitive fake, drive-letter and separator normalisation, component-wise (not string) prefix (`/a/wt` vs `/a/wt2`), longest-matching worktree wins; uses a fake `PathFacts` (FR-012, FR-014).
- [X] T004 [US1] Implement `paths.rs` (`PathFacts` trait, real impl over std canonicalisation of the deepest existing ancestor, containment helper) and register `pub mod worktree_guard` in `crates/micold-core/src/lib.rs` (after T003).
- [X] T005 [P] [US1] Write failing tests, then implement in `crates/micold-core/src/worktree_guard/mod.rs`, the types of `data-model.md` (`SessionBinding`, `GuardInput`, `ToolCall`, `GuardVerdict`, `DENY_MESSAGE`) and `ToolCall::from_payload` parsing `tool_name`/`tool_input` (`file_path` for Edit/Write/MultiEdit, `notebook_path` for NotebookEdit, Bash `command`; reads and unknown tools are `Other`; unparsable payload yields a fail-open reason). Deny message text per FR-003.

## Phase 3: User Story 1 - A session cannot work in another worktree (P1) 🎯 MVP

**Goal**: an Edit/Write/MultiEdit/NotebookEdit call that targets another worktree of the project is denied in every launched session, with the `create_session` message; the shared build dir and the prefetch worktree stay allowed (US4, folded in so the live guard cannot break builds or autopilot).

**Independent Test**: `mise run test-core` decision tests; daemon receiver tests; quickstart B steps 1 and 3.

- [X] T006 [US1] Write failing tests in `crates/micold-core/src/worktree_guard/mod.rs` for `decide` over edit tools: deny B from A (SC-001), allow own worktree, worktree session vs project root outside the allowlist denied, Default session vs any worktree denied, Default session in the root allowed, removed worktree allowed, `AllowUnknown` for unknown binding or empty list (FR-009), allowlist (`<root>/target-shared`, directory name starting `autopilot-prefetch`) allowed from any session (FR-007), message names the target, 50-worktree decision well under 100 ms (SC-005), same verdict for Windows, macOS and Linux layouts (SC-006).
- [X] T007 [US1] Implement `decide` and the allowlist constants (`AUTOPILOT_PREFETCH_PREFIX`, `target-shared` carve-out before the root test) in `worktree_guard/mod.rs` (after T004, T005, T006).
- [X] T008 [P] [US1] Write failing daemon tests: in `crates/micold-daemon/src/hooks.rs` (or its test module) `PreToolUse` on a guarded session answers 200 with the deny body of `contracts/pretooluse-guard.md`; an allowed call answers the unchanged empty 200; fail-open logs a warn naming the reason and never the body (hooks.md rule 5); `settings_json` for a guarded session still contains the `PreToolUse` hook entry and no `allow`/override keys (FR-013); a `DaemonState` test that the guard state built at launch carries the session's binding and project root (the frozen-after-toggle test is in T023, with the setting).
- [X] T028 [US1] Write a failing integration test `crates/micold-daemon/tests/worktree_guard_sessions.rs` (Constitution isolation gate; before T009): against a real catalog with two worktrees, a session in worktree A and a Default session each get the expected deny/allow over the receiver, and one session's state does not affect the other (FR-011, SC-001, Principle II and III).
- [X] T009 [US1] Implement in `crates/micold-daemon/src/hooks.rs`: a respond-with-JSON-body path, the `PreToolUse` branch calling core `decide`, `GuardState` (`Off` | `On{binding, project}`) passed through `prepare_settings` with the existing `StatusContext`; in `crates/micold-daemon/src/state.rs`: build `GuardState` at launch in `hook_settings_file` (own worktree or Default root, project root) and a `DaemonState` accessor that reads the project's reconciled worktree list from the catalog (no `git` per call). The guard is always `On` until M5 adds the setting (after T007, T008).
- [X] T010 [US1] Update `docs/user-guide/worktrees-and-sessions.md`: a session may only work in its own worktree, what the denial says, the shared build dir and prefetch allowance, and that other providers are not covered (R9).

## Phase 4: User Story 1 (Bash) - shell commands judged (P1)

**Goal**: a Bash call whose working directory, chained `cd`/`pushd` target or explicit path argument is in another worktree is denied (US1 scenario 2, FR-004, FR-011).

**Independent Test**: `mise run test-core` shell tests; daemon test of a Bash deny; quickstart B step 1 with a shell command.

- [x] T011 [P] [US1] Write failing tests for `crates/micold-core/src/worktree_guard/shell.rs`: quote-aware tokeniser, split on `&&`, `;`, `||`, newline; running directory tracked through `cd`/`pushd`; absolute, `./` and `../` path tokens judged; starting directory from the payload's top-level `cwd` (FR-011); variables, substitution and nested scripts are not analysed (documented limit, test that they allow).
- [x] T012 [US1] Implement `shell.rs` and wire `ToolCall::Bash` into `decide` in `worktree_guard/mod.rs`, with decision tests for `cd ../other && ls`, a Bash `cwd` inside another worktree, own-worktree and allowlist paths still allowed (after T011).
- [x] T013 [US1] Daemon test and wiring: `PreToolUse` Bash payload with `cwd` in another worktree gets the deny body (`crates/micold-daemon/src/hooks.rs`); the hook reads top-level `cwd` into `GuardInput.session_cwd` (after T012).
- [x] T014 [US1] Update `docs/user-guide/worktrees-and-sessions.md`: shell commands are judged best effort; expansion, variables and nested scripts are not analysed.

## Phase 5: User Story 2 - Worktree creation by other means is blocked (P1)

**Goal**: `EnterWorktree` and every `git worktree add` form are denied in launched sessions; `create_worktree` stays available (FR-005, FR-006, FR-013).

**Independent Test**: daemon test of the emitted `--settings` JSON; quickstart B step 2.

- [ ] T015 [P] [US2] Write failing tests: (a) `crates/micold-core/src/worktree_guard/shell.rs` denies a command whose tokenised git invocation has `worktree` then `add` after skipping `-C <dir>`, `-c k=v`, `--git-dir=` (R2), including chained and quoted forms; (b) `crates/micold-daemon/src/hooks.rs` `settings_json` emits `permissions.deny` containing exactly `EnterWorktree`, `Bash(git worktree add:*)`, `Bash(git * worktree add*)`, does not list `mcp__micold__create_worktree`, and has no `allow` key (FR-006, FR-013); (c) a core test matching the deny rule strings against `git worktree add` and `git -C . worktree add` (plan test strategy).
- [ ] T016 [US2] Implement the `git worktree add` analysis in `shell.rs` and the `permissions.deny` block in `settings_json` (after T015). Rules are static; nothing else in the file changes shape.
- [ ] T017 [US2] Update `docs/user-guide/agent-tools.md`: launched sessions cannot enter or add worktrees; `create_worktree` then `create_session` is the route.

## Phase 6: User Story 3 - `create_worktree` tells the agent to hand off (P2)

**Goal**: the `create_worktree` result carries the hand-off instruction (FR-010).

**Independent Test**: daemon test on the result text; quickstart B step 5.

- [ ] T018 [P] [US3] Write a failing test in `crates/micold-daemon/src/mcp/tools.rs` (or its test module): a successful `create_worktree` result has the existing JSON plus a `next_step` string, and its text content ends with "Do not work in this worktree from this session. Hand the work to a dedicated session with create_session for this worktree."
- [ ] T019 [US3] Implement the result change in `crates/micold-daemon/src/mcp/tools.rs` (after T018).
- [ ] T020 [US3] Update `docs/user-guide/agent-tools.md`: the `create_worktree` result and the hand-off step.

## Phase 7: User Story 5 - Opt out (P3)

**Goal**: one app-wide setting, default on, removes the guard and the deny rules for sessions launched after it is turned off (FR-008, R7, R8). (User Story 4 shipped in M1.)

**Independent Test**: `mise run test-core`, `cargo test -p micold-daemon`, client unit test; quickstart B step 4.

- [ ] T021 [P] [US5] Write failing core tests: `crates/micold-core/src/settings.rs` persisted `Settings.worktree_guard_enabled` defaults to true and a settings file without the key loads as true; `crates/micold-core/src/protocol/messages.rs` `DaemonSettings` field round-trips and the edit request carries `Option<bool>`; update `tests/protocol_roundtrip.rs`, `tests/settings_roundtrip.rs`, `tests/schema_hash.rs` and `protocol/version.rs` for the version bump.
- [ ] T022 [US5] Implement the setting in `settings.rs` and `protocol/messages.rs` (default true, serde default, mirror struct, protocol version bump), mirroring every site of `tool_server_enabled` in core, and fixing struct-literal breakage in every file `grep -rl tool_server_enabled crates` lists (after T021).
- [ ] T023 [P] [US5] Write failing daemon tests: `catalog.rs` getter/setter and `settings_wire`; `server.rs` edit-request handler applies `Option<bool>`; a launch with the setting off emits no guard response state and no `permissions.deny` (SC-004, FR-008); toggling after launch leaves a running session's `GuardState` unchanged (R7); a launch with it on is guarded (US5 scenario 2). Model on `tests/pr_status_setting.rs`.
- [ ] T024 [US5] Implement in `crates/micold-daemon/src/catalog.rs`, `server.rs`, `state.rs`: setting plumbing, `GuardState::Off` and omitted `permissions.deny` at launch when off, frozen per session; fix struct-literal breakage in the daemon tests and support files that `grep -rl tool_server_enabled crates/micold-daemon` lists (after T022, T023).
- [ ] T025 [P] [US5] Write a failing client test (`crates/micold-client/tests/features_settings.rs`, `src/main_tests.rs`, `tests/support/covered_states.rs` as `tool_server_enabled` is covered): the toggle row "Guard sessions to their own worktree" reflects and edits the setting.
- [ ] T026 [US5] Implement the toggle row and plumbing in `crates/micold-client/src/features/settings.rs`, `ui/settings/environment.rs`, `shell/persist.rs`, `features/session.rs`, `shell/startup.rs`, `shell/daemon_sync.rs`, reusing the existing settings toggle row (Constitution VIII; no new widget), beside "Let AI sessions manage worktrees and sessions" (after T024, T025).
- [ ] T027 [US5] Update `docs/user-guide/settings.md`: the "Guard sessions to their own worktree" setting, default on, applies to sessions launched after the change.

## Dependencies and order

- T001 done; T002 first in M1 (a negative result stops the run).
- T003 -> T004; T005 parallel to T003; T006 needs T005 types, T007 needs T004, T005, T006; T008 and T028 -> T009 needs T007.
- M2 needs M1 (`decide`, receiver); M3 needs M2 (`shell.rs`); M4 needs M1 (`mcp/tools.rs` independent); M5 needs M3 (deny rules to remove when off).
- Post-merge (close unit, no code): quickstart B results, ticking.

## Implementation Strategy

MVP is M1: edit/write tools denied across worktrees with the allowlist, on every launched session. M2 closes the shell route, M3 the worktree-creation route, M4 steers the agent, M5 adds the opt-out. Order keeps `main` green and every shipped milestone observable.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Edit and write guard with allowlist 🎯 MVP

- **Tasks**: T001–T010, T028
- **Deliverable**: a launched session's Edit/Write/MultiEdit/NotebookEdit call into another worktree is denied with the `create_session` message, while its own worktree, `target-shared/` and `autopilot-prefetch*` worktrees stay allowed, and the deny-body channel is proven against the real CLI.
- **Satisfies**: US1 acceptance scenarios 1, 3, 4, 5; US4 acceptance scenarios 1–2; FR-001 (always on until M5), FR-002 (edit tools), FR-003, FR-007, FR-009, FR-011 (edit tools; Bash `cwd` in M2), FR-012, FR-014; SC-001, SC-003, SC-005, SC-006
- **Verify**: `mise run test-core` (`worktree_guard`), `cargo test -p micold-daemon` (receiver deny), quickstart B steps 1 and 3; T002 recorded in research.md R1
- **Depends on**: —
- **Tier**: full

### M2 — Shell commands judged

- **Tasks**: T011–T014
- **Deliverable**: a Bash call whose working directory, chained `cd`/`pushd` or path argument is in another worktree is denied.
- **Satisfies**: US1 acceptance scenario 2; FR-002 (shell), FR-004, FR-011
- **Verify**: `cargo test -p micold-core worktree_guard::shell`; `cargo test -p micold-daemon` Bash deny test
- **Depends on**: M1
- **Tier**: full

### M3 — Worktree creation by other means blocked

- **Tasks**: T015–T017
- **Deliverable**: launched sessions cannot call `EnterWorktree` or run any `git worktree add` form; `create_worktree` still works.
- **Satisfies**: US2 acceptance scenarios 1–3; FR-005, FR-006, FR-013; SC-002, SC-007
- **Verify**: `cargo test -p micold-daemon` (emitted `--settings` JSON) and `cargo test -p micold-core worktree_guard`; quickstart B step 2
- **Depends on**: M2
- **Tier**: full

### M4 — `create_worktree` hand-off text

- **Tasks**: T018–T020
- **Deliverable**: the `create_worktree` result tells the agent not to work in the new worktree and to hand off with `create_session`.
- **Satisfies**: US3 acceptance scenario 1; FR-010
- **Verify**: `cargo test -p micold-daemon` result-text test; quickstart B step 5
- **Depends on**: M1
- **Tier**: full

### M5 — Opt-out setting

- **Tasks**: T021–T027
- **Deliverable**: Settings shows "Guard sessions to their own worktree" (default on); turning it off removes the guard and deny rules for newly launched sessions only.
- **Satisfies**: US5 acceptance scenarios 1–2; FR-008; SC-004
- **Verify**: `mise run test-core`, `cargo test -p micold-daemon`, `cargo test -p micold-client`; quickstart B step 4
- **Depends on**: M3
- **Tier**: full
