# Tasks: Switchable Regular Terminal Mode

**Input**: Design documents from `/specs/010-regular-terminal-mode/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md (all present)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY for every genuinely pure/testable unit of logic. Real-PTY-process glue in `src/main.rs`
/ `src/ui/terminal.rs` has no practical automated test in this codebase today (the existing
`RuntimeTerminal`/`spawn_pty` machinery from features 005/006 has none either — only the pure
`TerminalBackend`/`FakeTerminalBackend` seam is unit-tested); those tasks are called out
explicitly as quickstart-validated deviations, mirroring feature 006's `T012` precedent.

**Documentation**: Per Constitution Principle VII, every user-facing user story ships its
user-guide update in the same change.

**Cross-platform**: Per Constitution Principle VI, the one platform-varying piece (shell command
resolution, research R3) is isolated behind `default_shell_command` and covered by CI on Linux,
macOS, and Windows.

**Organization**: Tasks are grouped by user story (spec.md priorities: US1 P1, US2 P1, US3 P2).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: US1 / US2 / US3, per spec.md
- File paths are exact

---

## Phase 1: Setup

**Purpose**: The one genuinely new shared resource this feature needs before anything else can
reference it — two new icon glyphs (no other new dependency; research R7).

- [X] T001 [P] Add two new `Icon` variants (final names chosen here, e.g. `Icon::AiCli` /
  `Icon::RegularTerminal`) to `src/icons.rs` (`Icon` enum, `Icon::ALL`, `glyph()`), sourcing
  codepoints from the vendored `MaterialSymbolsOutlined[...].codepoints` reference (natural
  Material Symbols fits: `terminal`, and `smart_toy`/`robot_2` for the AI CLI); add both rows to
  `assets/fonts/PROVENANCE.md`'s codepoint table (research R7). Chosen + visually verified by
  rendering both glyphs: `Icon::AiCli` = `smart_toy` `U+F882`, `Icon::RegularTerminal` =
  `terminal` `U+EB8E`.
- [X] T002 Update `tests/icons.rs`'s pinned `expected()` codepoint table to regression-lock the
  two new icons chosen in T001 (depends on T001).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The two-process-per-session model (`TerminalMode`, `ShellLifecycle`,
`SessionTerminals`) every user story sits on top of. No user story is independently testable
until this phase is green.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

### Tests for Foundational (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and confirm they FAIL before implementation.

- [X] T003 [P] Write failing pure tests in `tests/session_terminal_mode.rs` (NEW file,
  `--no-default-features`): `TerminalMode::default() == AiCli`; `TerminalMode::other()` toggles
  both directions; `ShellLifecycle::default() == NotStarted`; `start_shell` is a no-op from
  `Starting`/`Running` and transitions `NotStarted|Exited → Starting`;
  `mark_shell_running`/`mark_shell_exited` transitions; `Session::start_new`/`restored` default
  `mode` to `AiCli` and `shell_lifecycle` to `NotStarted` (data-model.md).
- [X] T004 [P] Write failing pure tests in `tests/shell_command.rs` (NEW file,
  `--no-default-features`) for `default_shell_command(shell_env, comspec_env)`: non-empty env
  value wins; `None`/empty-string falls back per platform (contracts/shell-process.md).
- [X] T005 [P] Write failing pure tests in `tests/store_terminal_mode.rs` (NEW file,
  `--no-default-features`): a `StoredSession` JSON blob with no `mode` key deserializes as
  `AiCli` (back-compat); round-trip `AiCli`/`Regular` through `StoredCatalog::from_workspace` /
  `into_workspace` (contracts/persistence-schema.md).
- [X] T006 [P] Write failing pure tests in `tests/app_state.rs` (extend existing file,
  `--no-default-features`): `Message::TerminalModeToggled` flips `active_session`'s `mode` via
  `other()`; `Message::ShellSessionRunning(id)` sets that session's `shell_lifecycle` to
  `Running`; `Message::ShellSessionExited(id)` sets it to `Exited` (data-model.md).

### Implementation for Foundational

- [X] T007 [P] Add `TerminalMode` (+ `other()`) and `ShellLifecycle` (+ `is_active()`) enums to
  `src/session.rs` (data-model.md); makes the enum-shaped assertions in T003 pass.
- [X] T008 Extend the `Session` struct in `src/session.rs` with `mode: TerminalMode` and
  `shell_lifecycle: ShellLifecycle`; update `start_new`/`restored` signatures; add
  `set_mode`/`start_shell`/`mark_shell_running`/`mark_shell_exited` methods (depends on T007;
  completes T003).
- [X] T009 [P] Add the pure `default_shell_command(shell_env: Option<&str>, comspec_env:
  Option<&str>) -> String` function to `src/terminal.rs` (research R3); makes T004 pass.
- [X] T010 Extend `StoredSession` in `src/store.rs` with `#[serde(default)] mode:
  StoredTerminalMode`; wire `StoredCatalog::from_workspace` / `into_workspace` to round-trip it
  (depends on T008; makes T005 pass; FR-011; contracts/persistence-schema.md).
- [X] T011 Add `Message::TerminalModeToggled`, `Message::TerminalRestartRequested`,
  `Message::ShellSessionRunning(SessionId)`, `Message::ShellSessionExited(SessionId)` variants
  and their pure reducers to `src/app.rs` (depends on T008; makes T006 pass).
- [X] T012 Add `SessionTerminals { ai_cli: Option<RuntimeTerminal>, shell: Option<RuntimeTerminal>
  }` with `attached()`/`attached_mut()` to `src/ui/terminal.rs`; change `App.terminals` in
  `src/main.rs` from `HashMap<SessionId, RuntimeTerminal>` to `HashMap<SessionId,
  SessionTerminals>` and update the `TerminalTick` pump loop, `pane()`'s `RuntimeTerminal` render
  borrow, and the `TerminalBytes` write-through to go through the session's mode-selected slot
  (depends on T008, T011; data-model.md). Routing both slots through the *same* `TerminalPane`
  render/input path (rather than a Regular-mode-specific one) is what makes FR-008 (identical
  real-terminal behavior in both modes) true by construction; keying the map per-`SessionId`
  unchanged from today is what keeps SC-001/SC-005 (fast, per-session-isolated switching) true by
  construction too (FR-008, SC-001, SC-005).
- [X] T013 Factor `spawn_pty`'s PTY-open + `Term`-construction body (`src/ui/terminal.rs`) into a
  private helper shared with a new `spawn_shell_pty(cwd: &Path, env: &[(String, String)],
  scrollback_lines: usize) -> io::Result<RuntimeTerminal>`, built from `default_shell_command`
  with no extra args (depends on T009, T012; research R4, contracts/shell-process.md).
- [X] T014 Extend `handle_process_exits` in `src/main.rs` to scan both `SessionTerminals` slots
  per session: keep the existing `ai_cli` crash-loop branch (`on_unexpected_exit`) unchanged in
  behavior, now reading `st.ai_cli`; add a `shell` branch that removes the slot and dispatches
  `Message::ShellSessionExited(id)` with **no** restart attempt (depends on T012; research R6/R2,
  FR-013).
- [X] T015 Extend `SessionCloseRequested` handling in `src/main.rs` so both `SessionTerminals`
  slots (not just `ai_cli`) are killed/dropped on session close (depends on T012; FR-012, FR-014).
  Note: `Session::stop_for_project_change` (`src/session.rs`) currently has **no call sites** in
  `src/main.rs`/`src/app.rs` — feature 008 stopped invoking it when switching projects, so
  project deactivation does not tear down processes today and needs no change here; do not go
  looking for a call site that doesn't exist. `SessionCloseRequested` is the only real teardown
  trigger this task touches.

**Checkpoint**: `cargo test --no-default-features` and `cargo test --features gui` both pass; the
app builds and runs exactly as before (no user-visible toggle yet — every session still behaves
as AI-CLI-only, since nothing spawns a second process until a story wires the toggle).

---

## Phase 3: User Story 1 - Drop into a regular shell without leaving the session (Priority: P1) 🎯 MVP

**Goal**: A toggle in the terminal's bottom status bar switches the pane between the `claude`
process and a plain shell scoped to the session's worktree, without leaving the app.

**Independent Test**: Open a session with `claude` running, use the mode toggle to switch to
Regular Terminal mode, confirm a plain shell is running with cwd = the session's worktree, run a
shell command, and confirm it executes normally.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and confirm they FAIL before implementation.

- [X] T016 [P] [US1] Write a failing pure test in `tests/session_terminal_mode.rs` (extends
  T003's file): `mode_glyph(TerminalMode) -> Icon` and `mode_tooltip(TerminalMode) -> &'static
  str` return a distinct value for `AiCli` vs `Regular` (contracts/mode-toggle-ui.md's mapping
  table; FR-009).

### Implementation for User Story 1

- [X] T017 [US1] Add `mode_glyph`/`mode_tooltip` to `src/session.rs`; makes T016 pass.
- [X] T018 [US1] Wire `Message::TerminalModeToggled` end-to-end in `src/main.rs`:
  `core.update(TerminalModeToggled)` flips `session.mode`; if the newly-selected slot in
  `app.terminals[id]` is empty, spawn it (`spawn_pty(..., LaunchMode::Resume)` for `AiCli`,
  `spawn_shell_pty(...)` for `Regular`), insert into the slot, and follow up with
  `SessionRunning`/`ShellSessionRunning`; `persist(&app.core)` (FR-001, FR-003, FR-004, FR-007,
  FR-010). *(Real-process spawn-on-toggle has no practical unit test without a live PTY;
  validated by `quickstart.md` Scenario 1, mirroring feature 006's `T012` recorded deviation.)*
  Implemented as the shared `ensure_attached_process` helper (also used by `SessionSelected` and
  `TerminalRestartRequested`) rather than inline logic in the `TerminalModeToggled` arm — same
  behavior, one spawn-or-reattach path for all three callers.
- [X] T019 [US1] Add the toggle `IconButton` (`src/ui/material/icon_button.rs`) to the bottom bar
  in `pane()` (`src/ui/terminal.rs`), glyph from `mode_glyph(session.mode)`,
  `.on_press(Message::TerminalModeToggled)` unconditional — no disabled state (FR-001, FR-002;
  contracts/mode-toggle-ui.md).
- [X] T020 [US1] Add the manual restart control to the same bottom bar, shown exactly when the
  attached process is not running (predicate in contracts/terminal-mode-lifecycle.md), dispatching
  `Message::TerminalRestartRequested`; wire its `src/main.rs` handler to the same spawn logic as
  T018, addressed at the current mode's slot (FR-013).
- [X] T021 [P] [US1] Document the mode toggle — what it does, that the shell is scoped to the
  worktree, that both processes keep running across a switch — in
  `docs/user-guide/worktrees-and-sessions.md` (Principle VII).

**Checkpoint**: A user can toggle to a scoped shell, run commands, toggle back, and — if the
shell exits — restart it via the new control. MVP demoable.

---

## Phase 4: User Story 2 - The `claude` conversation survives round-trips (Priority: P1)

**Goal**: Switching away from and back to AI CLI mode never restarts or loses the `claude`
conversation, even mid-turn or across an independent background crash.

**Independent Test**: Start a `claude` conversation, exchange a message, switch to Regular
Terminal mode, run a command, switch back, and confirm the same conversation (same session id,
full history, no interruption to an in-flight turn) is shown.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and confirm they FAIL before implementation.

- [X] T022 [US2] Write a failing pure test in `tests/session_terminal_mode.rs` (extends the same
  file as T003/T016 — not marked `[P]` relative to T016, since both edit that file): `set_mode`,
  `start_shell`, `mark_shell_running`, and `mark_shell_exited` never mutate `Session.lifecycle`
  (the AI CLI's own state) — the two lifecycles are independent by construction (FR-006; this is
  the pure-model half of "the conversation survives round-trips").

### Implementation for User Story 2

- [X] T023 [US2] In `src/main.rs`'s `TerminalModeToggled` handler (T018), confirm/adjust that
  switching to `AiCli` mode reattaches an already-populated `ai_cli` slot as-is (no spawn call at
  all) and only spawns with `LaunchMode::Resume` when that slot is empty — i.e. a still-running
  `claude` process is never re-spawned by a mode switch (FR-005, FR-006). Confirmed: this was
  already true of `ensure_attached_process` (T018) — `already_attached` short-circuits before any
  spawn call, so no change was needed here.
- [X] T024 [US2] Code-review confirmation (no behavior change expected): `handle_process_exits`'s
  `ai_cli` branch (T014) calls `session.on_unexpected_exit()` unconditionally on `Session.mode` —
  the crash-loop guard itself is already exhaustively covered by the existing
  `tests/session_crash_restart.rs` (feature 005) and does not need re-testing; only that the new
  dual-slot scan still reaches it while `Regular` mode is displayed is new (research R6).
  *(Background-crash-restart while backgrounded has no practical automated test without a live
  process exit; validated by `quickstart.md` Scenario 2.)* Confirmed by reading
  `handle_process_exits` (`src/main.rs`): the `ai_cli_exited` loop calls `on_unexpected_exit()`
  unconditionally, with no read of `session.mode` anywhere in that branch.
- [X] T025 [P] [US2] Document that switching to Regular Terminal mode never stops, restarts, or
  otherwise affects the `claude` conversation — including mid-turn — in
  `docs/user-guide/worktrees-and-sessions.md` (Principle VII).

**Checkpoint**: The `claude` conversation demonstrably survives any number of mode round-trips,
including while mid-turn and across an independent crash-restart.

---

## Phase 5: User Story 3 - Always know which process is listening (Priority: P2)

**Goal**: The toggle button's icon and tooltip are the single, always-current source of "which
process are my keystrokes going to" — no separate indicator to drift out of sync.

**Independent Test**: Toggle between modes and confirm the bottom-bar button's icon/tooltip alone
identifies the active mode at all times, updating immediately on switch.

### Tests for User Story 3

> No new automated test: T016 (US1) already proves the underlying `mode_glyph`/`mode_tooltip`
> mapping is total and distinct per variant, and the button re-derives its glyph from
> `session.mode` on every render (T019) rather than caching it — so "always current" is a
> consequence of that mapping being pure, not a separately testable property. "Immediately
> updates" in the running app is validated by `quickstart.md` Scenario 3, per the same
> no-practical-unit-test precedent as the other GUI-timing checks in this feature.

### Implementation for User Story 3

- [X] T026 [US3] Wrap the toggle button from T019 in `Tooltip::new(..., mode_tooltip(session.mode),
  roles)` (`src/ui/material/mod.rs`) with the exact copy from contracts/mode-toggle-ui.md's table
  (FR-009). Already done as part of T019's edit — the toggle was built directly with
  `Tooltip::new(IconButton::new(mode_glyph(mode), r).on_press(...), mode_tooltip(mode), r)` in
  `pane()` (`src/ui/terminal.rs`), rather than as a separate wrapping step.
- [X] T027 [P] [US3] Document the mode indicator (icon + tooltip is the one place to check, always
  current, no separate indicator element) in `docs/user-guide/worktrees-and-sessions.md`
  (Principle VII).

**Checkpoint**: Mode is unambiguous at a glance in both states.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that span all three stories.

- [X] T028 [P] Cross-cutting documentation review and index/navigation updates in `docs/`.
  Reviewed all `docs/user-guide/worktrees-and-sessions.md` additions (T021/T025/T027) for a
  consistent voice with the surrounding sections, and updated `docs/README.md`'s Worktrees &
  Sessions summary line to mention the shell-toggle capability.
- [X] T029 Verify `cargo build`/`cargo test` (both `--no-default-features` and `--features gui`)
  pass on Linux, macOS, and Windows (Principle VI). Verified on Linux (this environment):
  `cargo fmt --check`, `cargo build`/`--features gui`, `cargo test --no-default-features`/
  `--features gui` (all suites, 0 failures), and `cargo clippy --features gui --lib --bins` /
  the four new/extended test files, all with `-D warnings` — all clean. macOS/Windows are CI-only
  in this environment (no local toolchain here); `default_shell_command`'s Windows branch
  (`tests/shell_command.rs`) is `#[cfg(windows)]`-gated so it compiles and runs for real the next
  time CI executes on a Windows runner, per Principle VI and this file's cross-platform note.
- [X] T030 Run `quickstart.md` end-to-end (all 6 scenarios) as final manual validation. Confirmed
  each scenario against the implementation by inspection (no interactive-GUI tooling available in
  this environment to drive mouse/keyboard through the running app):
  Scenario 1 (toggle + real-terminal parity) ⟸ both modes render through the same `TerminalPane`
  (`pane()`, `src/ui/terminal.rs`); Scenario 2 (`claude` survives round-trips) ⟸ T022's isolation
  test + `ensure_attached_process`'s reattach-without-respawn path; Scenario 3 (mode always
  visible) ⟸ the icon/tooltip re-derived from `session.mode` every render; Scenario 4 (shell exit
  + manual restart, no auto-retry) ⟸ `ShellLifecycle::mark_exited`'s lack of a restart decision
  and `attached_process_restartable`'s predicate; Scenario 5 (per-session independence) ⟸
  `SessionTerminals`/`Session.mode` both keyed per `SessionId`; Scenario 6 (mode persists across
  restart) ⟸ `store_terminal_mode.rs`'s roundtrip tests. `cargo run --features gui` launches
  cleanly in this environment (a display and the `claude` CLI are both present); a hands-on
  interactive pass through the running app is recommended before merging, as this codebase has no
  existing precedent for automating that (mirrors feature 006's `T012`).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately.
- **Foundational (Phase 2)**: No hard dependency on Setup — T001's icons are only needed by
  US1/US3, not by any of Foundational's own tasks (T003–T015), so Phase 1 and Phase 2 can run
  concurrently. Foundational itself BLOCKS all user stories.
- **User Stories (Phase 3+)**: All depend on Foundational (Phase 2) completion.
  - US1 (P1) and US2 (P1) can proceed in parallel once Foundational is done (different concerns:
    US1 = the shell side of the toggle, US2 = the AI-CLI-survives side) — but both touch the same
    `TerminalModeToggled` handler in `src/main.rs` (T018/T023), so in practice implement US1's
    T018 first, then US2's T023 refines it. Treat US1 → US2 as effectively sequential despite the
    priority tie.
  - US3 (P2) depends on US1's T019 (the button must exist before it can be wrapped in a tooltip).
- **Polish (Phase 6)**: Depends on all three user stories being complete.

### User Story Dependencies

- **US1 (P1)**: Foundational only.
- **US2 (P1)**: Foundational + US1's T018 (extends the same handler).
- **US3 (P2)**: Foundational + US1's T019 (wraps the same button).

### Parallel Opportunities

- T001 (icons) can run alongside all of Phase 2.
- Within Foundational: T003–T006 (tests) in parallel (4 different files); then T007/T009 in
  parallel (different files); T010/T011 wait on T008; T012 waits on T008+T011; T013 waits on
  T009+T012; T014 and T015 both wait on T012 but both edit `src/main.rs` — run them sequentially,
  not in parallel, despite neither depending on the other's output.
- T016 (US1 test) and T022 (US2 test) are NOT parallel with each other — both edit
  `tests/session_terminal_mode.rs`. They can each start as soon as Foundational is green, but
  write them one after the other (T016 first, since US1 precedes US2).
- T021, T025, T027 (docs) can each run in parallel with the next story's tests once their own
  story's implementation tasks are done (different files: `docs/user-guide/worktrees-and-sessions.md`
  vs. the next story's test file).

---

## Parallel Example: Foundational Tests

```bash
# Launch all Foundational test-writing tasks together:
Task: "Write failing pure tests in tests/session_terminal_mode.rs"
Task: "Write failing pure tests in tests/shell_command.rs"
Task: "Write failing pure tests in tests/store_terminal_mode.rs"
Task: "Write failing pure tests in tests/app_state.rs (extend)"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (icons).
2. Complete Phase 2: Foundational (CRITICAL — blocks everything).
3. Complete Phase 3: User Story 1.
4. **STOP and VALIDATE**: `quickstart.md` Scenario 1 — toggle works, shell state persists across
   switches.
5. Demo if ready — this alone delivers the feature's headline value (drop into a shell without
   leaving the app).

### Incremental Delivery

1. Setup + Foundational → two-process infrastructure ready, no user-visible change yet.
2. Add US1 → toggle + shell works → validate Scenario 1 (MVP!).
3. Add US2 → `claude` survival guarantee explicit + validated → Scenario 2.
4. Add US3 → indicator polish → Scenario 3.
5. Polish → cross-platform verification + full quickstart pass.

---

## Notes

- [P] tasks = different files, no dependencies.
- [Story] label maps task to specific user story for traceability.
- US1 and US2 share `src/main.rs`'s `TerminalModeToggled` handler — despite both being P1, build
  US1's version first (T018) and let US2 (T023) refine it, rather than parallelizing that one
  file.
- Several tasks are explicitly marked as having "no practical automated test" for real-PTY-process
  glue — this mirrors existing, established precedent in this codebase (feature 006's `T012`), not
  a new exception. Every genuinely pure decision (enum transitions, persistence shape, message
  reducers, the icon/tooltip mapping, lifecycle independence) IS tested first per Constitution
  Principle I.
- Commit after each task or logical group; verify tests fail before implementing, then pass after.
