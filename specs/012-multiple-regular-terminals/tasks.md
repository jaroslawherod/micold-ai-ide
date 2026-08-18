# Tasks: Multiple Regular Terminal Instances per Session

**Input**: Design documents from `/specs/012-multiple-regular-terminals/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md (all present)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY for every genuinely pure/testable unit of logic. Real-PTY-process glue in `src/main.rs`
has no practical automated test in this codebase today — the existing single-shell machinery
from features 005/006/010 has none either. Those tasks are called out explicitly as
quickstart-validated deviations, mirroring feature 010's `T018` precedent. Every pure decision
this feature adds (`ShellInstanceId` allocation, the `Session` instance mutators, the new keymap
chord) IS tested first.

**Documentation**: Per Constitution Principle VII, every user-facing user story ships its
user-guide update in the same change.

**Cross-platform**: Per Constitution Principle VI, the one platform-varying piece (the
`Ctrl+Shift+T`/`Cmd+Shift+T` chord, research R4) is isolated behind `is_new_terminal_chord` and
covered by CI on Linux, macOS, and Windows.

**Organization**: Tasks are grouped by user story (spec.md priorities: US1 P1, US2 P1, US3 P2,
US4 P3).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: US1 / US2 / US3 / US4, per spec.md
- File paths are exact

---

## Phase 1: Setup

**Purpose**: The one new shared resource this feature needs before any story references it — one
new icon glyph for the "open a new instance" affordance (no other new dependency).

- [X] T001 [P] Add `Icon::AddTerminalInstance` to `src/icons.rs` (`Icon` enum, `Icon::ALL`,
  `glyph()`) (plan.md Constitution Check, Principle VIII). **Deviation from plan**: the plan
  assumed reusing `AddSession`'s `add` glyph (`U+E145`), but `tests/icons.rs` enforces "no two
  icons share a codepoint" and `AddWorktree` already proved distinct "add" concepts get distinct
  glyphs in this codebase — reusing `E145` would have broken that invariant. Looked up a genuinely
  distinct glyph (`add_box`, `U+E146`, adjacent to `add` in the Material Symbols codepoint space)
  from the upstream codepoints manifest instead; recorded in `assets/fonts/PROVENANCE.md`.
- [X] T002 Update `tests/icons.rs`'s pinned `expected()` codepoint table and the `Icon::ALL.len()`
  assertion (24 → 25) to include `Icon::AddTerminalInstance` (depends on T001).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The multi-instance data model (`ShellInstanceId`, `ShellInstance`, `Session.shells`/
`active_shell`) and its `SessionTerminals` counterpart every user story sits on top of — including
re-pointing the *existing* single-instance toggle/restart/exit-detection code paths at the new
shape so today's behavior keeps working unchanged before any new capability is added.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

### Tests for Foundational (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and confirm they FAIL before implementation.

- [X] T003 [P] Write failing pure tests in `tests/session_shell_instances.rs` (NEW file,
  `--no-default-features`): `ShellInstanceId` allocation is monotonic and never reused across an
  open+close cycle; `open_shell_instance` appends to the end of `shells`, advances the new
  instance's `lifecycle` to `Starting`, and sets `active_shell` to the new id; `select_shell` is a
  no-op when given an id absent from `shells`; `close_shell` reassigns `active_shell` to the
  element now at the removed position (the former next instance) or, if none, the new last
  element — but **only** when the closed instance was `active_shell`, leaving `active_shell`
  untouched when a non-active instance is closed — and sets `mode` to `TerminalMode::AiCli`
  exactly when `shells` becomes empty; `restart_shell_instance`/`mark_shell_running(id)`/
  `mark_shell_exited(id)` are no-ops when `id` is absent, else delegate to that instance's
  unchanged `ShellLifecycle` transitions; `Session::start_new`/`restored` initialize `shells:
  vec![]`, `active_shell: None`, `next_shell_id: 1` (contracts/shell-instance-lifecycle.md,
  data-model.md).

### Implementation for Foundational

- [X] T004 Add `ShellInstanceId(pub u32)` and `ShellInstance { id: ShellInstanceId, lifecycle:
  ShellLifecycle }` to `src/session.rs` (data-model.md); makes the shape assertions in T003
  compile.
- [X] T005 Extend `Session` in `src/session.rs`: replace `shell_lifecycle: ShellLifecycle` with
  `shells: Vec<ShellInstance>`, `active_shell: Option<ShellInstanceId>`, `next_shell_id: u32`;
  update `start_new`/`restored` accordingly; add `open_shell_instance`, `select_shell`,
  `close_shell`, `restart_shell_instance`, `mark_shell_running(id)`, `mark_shell_exited(id)`,
  `active_shell_lifecycle()` methods exactly per contracts/shell-instance-lifecycle.md (depends
  on T004; makes T003 pass).
- [X] T006 Add `Message::ShellInstanceRunning(SessionId, ShellInstanceId)` and
  `Message::ShellInstanceExited(SessionId, ShellInstanceId)` to `src/app.rs`, with pure reducers
  calling `session.mark_shell_running(id)`/`mark_shell_exited(id)`; remove the now-superseded
  feature-010 `ShellSessionRunning(SessionId)`/`ShellSessionExited(SessionId)` variants and their
  call sites (depends on T005; data-model.md).
- [X] T007 Change `SessionTerminals.shell: Option<RuntimeTerminal>` to `shells:
  HashMap<ShellInstanceId, RuntimeTerminal>` in `src/ui/terminal.rs`; update `attached()`/
  `attached_mut()` to take `active_shell: Option<ShellInstanceId>` for the `Regular` arm; update
  `each_mut()`/`kill_all()` to iterate every shell entry instead of one `Option`; add `fn
  close_shell(&mut self, id: ShellInstanceId)` (kill + remove exactly one entry) (depends on T005;
  data-model.md).
- [X] T008 Update every remaining `App.terminals` call site in `src/main.rs` for the new shape:
  the `TerminalTick` pump loop (via `each_mut`, already generic over the collection), `pane()`'s
  `RuntimeTerminal` render borrow (`App::attached_terminal`/`attached_terminal_mut`), and the
  `TerminalBytes` write-through — all three now thread `session.active_shell` through
  `attached`/`attached_mut` alongside `session.mode` (depends on T006, T007).
- [X] T009 Adapted `ensure_attached_process`'s `Regular` branch (`src/main.rs`, renamed
  `session_cwd_mode_and_active_shell` alongside it) to the new shape: if `active_shell` is
  `None`, call `session.open_shell_instance()` for the new id, then `spawn_shell_pty` and insert
  under that id (today's "lazily start the shell on first switch into Regular mode," now
  id-addressed); otherwise reattach whichever instance `active_shell` already names, spawning
  only if that instance's process is currently absent. Observable behavior for a session that
  only ever has one instance is unchanged from feature 010 (depends on T007; FR-007's baseline).
- [X] T010 Adapted `handle_process_exits`'s shell branch (`src/main.rs`) to the new shape.
  **Scope note**: implemented directly to scan **every** entry of `st.shells` (not just the
  active one), which is the full FR-008/FR-009 (US4) capability rather than the narrower
  active-only baseline originally scoped here — see T027's note; both were done in the same
  continuous implementation pass, so there was no reason to write the narrower version first only
  to widen it again minutes later. Each exited instance is independently marked `Exited` via
  `session.mark_shell_exited(id)`; no restart decision.
- [X] T011 Adapted the existing `Message::TerminalRestartRequested` handler's `Regular` branch:
  it already only ever called the shared `ensure_attached_process` (T009), so T009 alone fully
  covers this — no separate edit was needed (mirrors feature 010 tasks.md's T023 precedent of a
  "confirmed, already true" note). Restarting a specific *background* instance directly
  (independent of `active_shell`) is `Message::ShellInstanceRestartRequested`, added in T028.
- [X] T012 Update `Message::SessionCloseRequested` handling in `src/main.rs` so
  `SessionTerminals::kill_all()` kills the AI CLI process plus **every** shell map entry, not just
  one (depends on T007; FR-018). **No separate edit needed**: `SessionCloseRequested`'s handler
  already just calls `st.kill_all()` unconditionally (unchanged from feature 010) — T007's
  `kill_all()` rewrite (iterating `self.shells.drain()` instead of a single `Option::take()`)
  already made this call site correct for any number of instances with no call-site change.

**Checkpoint**: `cargo test --no-default-features` and `cargo test --features gui` both pass; the
app builds and behaves exactly as it does today for any session that only ever has 0 or 1 Regular
Terminal instance — no user-visible change yet (no "+" control, no switcher row exists until
US1/US2 add them).

---

## Phase 3: User Story 1 - Run more than one shell at once in a session (Priority: P1) 🎯 MVP

**Goal**: From a session already showing one Regular Terminal instance, a user can open an
additional, fully independent shell instance — via an on-screen "+" control or the
`Ctrl+Shift+T`/`Cmd+Shift+T` shortcut — without disturbing the first instance or the AI CLI
process.

**Independent Test**: With a session already showing one Regular Terminal instance, open a
second instance, confirm both are independent shell processes scoped to the session's working
directory, and confirm a running command in one is unaffected by input typed into the other.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and confirm they FAIL before implementation.

> No new pure test is needed for `open_shell_instance` itself — its append/id-allocation/
> `active_shell`-assignment behavior is already exhaustively covered by Foundational's T003. This
> story only adds a new *trigger* (the "+" control and the keyboard shortcut) for a pure method
> that already exists and is already tested.

- [X] T013 [P] [US1] Write failing pure tests in `tests/keymap.rs` (extend existing file):
  `is_new_terminal_chord` detects `Ctrl+Shift+T` on non-macOS / `Cmd+Shift+T` on macOS; does not
  fire on plain `t`, `T`, or `Shift+t`; `KeyOutput::NewTerminalInstance` takes precedence over
  printable-character handling in `encode()` (contracts/keyboard-shortcut.md).

### Implementation for User Story 1

- [X] T014 [US1] Add `Message::ShellInstanceOpenRequested` to `src/app.rs` (no pure reducer body
  — mirrors `TerminalRestartRequested`/`SessionStartRequested`, which only trigger binary-side
  spawn logic).
- [X] T015 [US1] Wire `Message::ShellInstanceOpenRequested` end-to-end in `src/main.rs`: no-op if
  the active session's `mode != TerminalMode::Regular` (FR-019 edge case — the control/shortcut
  does nothing outside Regular mode, and does not switch modes); otherwise
  `session.open_shell_instance()` for the new id (mirrors `SessionStartRequested`'s direct
  `Session::start_new` construction), `spawn_shell_pty`, insert into
  `app.terminals[id].shells[shell_id]`, follow up with `Message::ShellInstanceRunning`,
  `persist(&app.core)` (depends on T009, T014; FR-001–FR-003, FR-007; contracts/shell-instance-
  lifecycle.md). *(No practical automated test for the real-PTY spawn path; validated by
  quickstart.md Scenario 1, mirroring feature 010's T018 precedent.)*
- [X] T016 [US1] Add the "open new instance" `IconButton` (`Icon::AddTerminalInstance`, T001) +
  `Tooltip` to the bottom bar in `pane()` (`src/ui/terminal.rs`), visible whenever
  `session.mode == TerminalMode::Regular` regardless of instance count,
  `.on_press(Message::ShellInstanceOpenRequested)` (FR-001, FR-005's always-visible "+" half;
  contracts/terminal-instance-switcher-ui.md).
- [X] T017 [US1] Add `is_new_terminal_chord`/`KeyOutput::NewTerminalInstance` to `src/keymap.rs`,
  checked in `encode()` at the same precedence tier as `is_release_chord` (depends on T013's
  tests; makes T013 pass; contracts/keyboard-shortcut.md). Also extended the parallel pure
  `KeyRouting`/`route_key` focus-routing model (`src/app.rs`, `contracts/focus-model.md`) with a
  matching `NewTerminalInstance` variant, since it exhaustively mirrors every `KeyOutput` variant
  and would not otherwise compile.
- [X] T018 [US1] Add the `KeyOutput::NewTerminalInstance → Message::ShellInstanceOpenRequested`
  match arm to `TerminalPane`'s key handler (`src/ui/material/terminal_pane.rs`) (depends on
  T017; FR-019).
- [X] T019 [P] [US1] Document opening additional Regular Terminal instances — the "+" control,
  the `Ctrl+Shift+T`/`Cmd+Shift+T` shortcut, each instance's own scoped working directory — in
  `docs/user-guide/worktrees-and-sessions.md` (Principle VII).

**Checkpoint**: A user can open a second (and further) fully independent shell instance from an
existing Regular Terminal pane and run commands in each without interference. MVP demoable.

---

## Phase 4: User Story 2 - See and switch between all open terminal instances (Priority: P1)

**Goal**: A user can glance at every open Regular Terminal instance for a session and jump
directly to any one of them.

**Independent Test**: With three or more Regular Terminal instances open for a session, use the
instance-switching control to select each one in turn and confirm the visible pane shows the
correct instance's process and output each time.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

> No new pure test is needed for `select_shell` itself — its guarded no-op-on-unknown-id behavior
> is already covered by Foundational's T003. The switcher row's visibility/click wiring is
> gui-only glue with no practical automated test in this codebase; validated by quickstart.md
> Scenario 2.

### Implementation for User Story 2

- [X] T020 [US2] Add `Message::ShellInstanceSelected(ShellInstanceId)` to `src/app.rs` with pure
  reducer `session.select_shell(id)` for `active_session`.
- [X] T021 [US2] Add the instance-switcher row to the bottom bar in `pane()`
  (`src/ui/terminal.rs`), visible only when `session.shells.len() > 1`, one entry per
  `ShellInstance` labeled by its `ShellInstanceId`'s numeric value in list order, the entry
  matching `session.active_shell` visually highlighted, `.on_press(Message::
  ShellInstanceSelected(entry.id))` per entry (depends on T020; FR-004, FR-005's list-portion,
  SC-004; contracts/terminal-instance-switcher-ui.md).
- [X] T022 [P] [US2] Document seeing and switching between multiple open Regular Terminal
  instances — the switcher row, what "active" means, per-session independence — in
  `docs/user-guide/worktrees-and-sessions.md` (Principle VII).

**Checkpoint**: A user can open several instances (US1) and freely switch among them via the
switcher row, with the previously-visible instance continuing to run in the background.

---

## Phase 5: User Story 3 - Close one instance without disturbing the rest (Priority: P2)

**Goal**: A user can close an individual Regular Terminal instance and have every sibling
instance and the AI CLI process keep running exactly as they were.

**Independent Test**: With three open Regular Terminal instances, close one that is not currently
visible, then close the one that is currently visible, confirming in each case that the
remaining sibling instances and the AI CLI process are unaffected.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

> No new pure test is needed for `close_shell` itself — its FR-012/FR-013 fallback logic
> (next-in-list, else previous, else revert to `AiCli` when empty) is already exhaustively
> covered by Foundational's T003. This story only wires its UI trigger; validated end-to-end by
> quickstart.md Scenario 3.

### Implementation for User Story 3

- [X] T023 [US3] Add `Message::ShellInstanceCloseRequested(ShellInstanceId)` to `src/app.rs` with
  pure reducer `session.close_shell(id)` (may flip `mode` to `AiCli` per FR-013).
- [X] T024 [US3] Wire `Message::ShellInstanceCloseRequested(id)` end-to-end in `src/main.rs`:
  `app.terminals.get_mut(&session_id)`'s `close_shell(id)` (T007 — kills + removes that one
  `RuntimeTerminal`), then `core.update(...)` (T023's pure reducer); if `mode` flipped to `AiCli`
  as a result, reattach the AI CLI process via the same `ensure_attached_process` path the
  primary toggle already uses; `persist(&app.core)` (depends on T023; FR-011–FR-013, FR-018).
  *(Real-process kill path; validated by quickstart.md Scenario 3.)*
- [X] T025 [US3] Add each switcher-row entry's close action (`src/ui/terminal.rs`, reusing
  `Icon::Delete` per feature 008's existing close-affordance precedent) dispatching
  `Message::ShellInstanceCloseRequested(entry.id)` (depends on T021, T023;
  contracts/terminal-instance-switcher-ui.md).
- [X] T026 [P] [US3] Document closing an individual instance — siblings unaffected, closing the
  currently-visible one falls back to the next/previous instance, closing the last one reverts to
  AI CLI mode — in `docs/user-guide/worktrees-and-sessions.md` (Principle VII).

**Checkpoint**: Instances can be closed individually, including the currently-visible one, with
the spec's resolved next-in-list/previous/AiCli-fallback behavior all observable.

---

## Phase 6: User Story 4 - Each instance's lifecycle and restart are independent (Priority: P3)

**Goal**: Each Regular Terminal instance's shell lifecycle (running/exited) and restart are
tracked and actioned fully independently of every sibling instance and the AI CLI process — even
for a background instance that is not currently attached to the pane.

**Independent Test**: With multiple Regular Terminal instances open, cause one to exit (e.g.,
type `exit`), confirm it shows a not-running state with a manual restart affordance while
siblings and the AI CLI process are unaffected, then restart just that instance and confirm it
resumes as a fresh shell without touching any sibling.

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

> No new pure test is needed — `restart_shell_instance`/`mark_shell_exited`'s id-addressed
> transitions are already covered by Foundational's T003. The multi-instance exit-detection scan
> and independent-restart wiring are gui/binary glue with no practical automated test in this
> codebase; validated by quickstart.md Scenario 4.

### Implementation for User Story 4

- [X] T027 [US4] Extend `handle_process_exits`'s shell branch (`src/main.rs`) to scan **every**
  entry of `st.shells`, not just `session.active_shell`'s — each independently detected and
  marked `Exited` via `session.mark_shell_exited(id)`, with no restart decision, regardless of
  whether that instance is the one currently attached to the pane (FR-008, FR-009). **Already
  done**: implemented directly as part of T010 in the same continuous pass (see T010's note) —
  no separate edit was needed here.
- [X] T028 [US4] Add `Message::ShellInstanceRestartRequested(ShellInstanceId)` to `src/app.rs` (no
  pure reducer body, mirrors `TerminalRestartRequested`) and wire it end-to-end in `src/main.rs`:
  the same spawn-if-`NotStarted`/`Exited` logic as T011's baseline, but addressable for **any**
  instance — including one that is not currently attached to the pane — so a background instance
  can be restarted without first switching to it (depends on T011; FR-010).
- [X] T029 [US4] Add a per-entry restart affordance to the switcher row (`src/ui/terminal.rs`),
  shown exactly when that entry's own `lifecycle ∈ {NotStarted, Exited}` (contracts/shell-
  instance-lifecycle.md's per-instance predicate), dispatching
  `Message::ShellInstanceRestartRequested(entry.id)` (depends on T021, T028).
- [X] T030 [P] [US4] Document that each instance's shell lifecycle and restart are fully
  independent — a background instance exiting or being restarted never affects siblings or the
  AI CLI process — in `docs/user-guide/worktrees-and-sessions.md` (Principle VII).

**Checkpoint**: All four user stories are independently functional; a background instance can
exit and be restarted without ever becoming the visible pane.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that span all four stories.

- [X] T031 [P] Cross-cutting documentation review and index/navigation updates in `docs/` —
  reviewed T019/T022/T026/T030's additions to `docs/user-guide/worktrees-and-sessions.md`: voice
  and bullet cadence are consistent with the surrounding feature-010 "Switching to a regular
  terminal" section. Updated `docs/README.md`'s Worktrees & Sessions summary line, which
  previously described only "toggling a session's terminal to a plain shell" (singular).
- [X] T032 Verify `cargo build`/`cargo test` (both `--no-default-features` and `--features gui`)
  and `cargo clippy` (both configs, `--all-targets`) pass on Linux, macOS, and Windows
  (Principle VI). Verified on Linux (this environment): `cargo fmt --check`, both build configs,
  both `cargo test --all-targets` runs (0 failures across the full suite, including the new
  `session_shell_instances`/extended `keymap`/`terminal_focus`/`app_state`/`session_terminal_mode`
  files), and `cargo clippy --all-targets` for both configs — all clean, no warnings. macOS/
  Windows are CI-only in this environment (no local toolchain); the one platform-varying piece
  added by this feature, `is_new_terminal_chord`'s `cfg(target_os = "macos")` split
  (`src/keymap.rs`), mirrors `is_release_chord`'s existing, already-CI-covered pattern exactly,
  and `tests/keymap.rs`'s new chord tests are themselves `cfg`-gated per platform so both branches
  compile and run for real the next time CI executes on each OS.
- [X] T033 Run `quickstart.md` end-to-end (all 7 scenarios) as final manual validation. Confirmed
  each scenario against the implementation by inspection (no interactive-GUI automation tooling
  available in this environment to drive mouse/keyboard through the running app, mirroring
  feature 010's `T030` precedent): Scenario 1 (open a second instance) ⟸
  `Session::open_shell_instance` (T005) + `ShellInstanceOpenRequested` wiring (T015); Scenario 2
  (see/switch) ⟸ the switcher row's `len() > 1` gate + `select_shell`/`ShellInstanceSelected`
  (T020/T021) and `ensure_attached_process`'s active-shell reattach (T009); Scenario 3 (close
  individually, next/previous/AiCli fallback) ⟸ `close_shell`'s position-based reassignment
  (T005, exhaustively covered by `tests/session_shell_instances.rs`) + T024's kill/reattach
  wiring; Scenario 4 (independent lifecycle/restart) ⟸ `handle_process_exits` scanning every
  `shells` entry (T010) + `ShellInstanceRestartRequested` addressable per-id (T028); Scenario 5
  (shortcut + AI-CLI-mode no-op) ⟸ `is_new_terminal_chord` (T017) + T015's `mode ==
  TerminalMode::Regular` gate; Scenario 6 (per-session independence) ⟸ `shells`/`active_shell`
  on `Session`, `SessionTerminals.shells` keyed per `SessionId`, unchanged from feature 010's
  per-session keying; Scenario 7 (reopen after restart) ⟸ `Session::restored` always
  initializing `shells: vec![]` (T005), consistent with FR-017. `cargo run --features gui`
  launches cleanly in this environment (a display is present) for a 5-second smoke check with no
  panic; a hands-on interactive pass through the running app (opening/switching/closing multiple
  instances, the keyboard shortcut) is recommended before merging.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately, and can run concurrently with
  Foundational (T001's icon is only needed by US1's T016, not by any Foundational task).
- **Foundational (Phase 2)**: BLOCKS all user stories — every story reads or writes
  `Session.shells`/`active_shell` or `SessionTerminals.shells`.
- **User Stories (Phase 3+)**: All depend on Foundational completion.
  - US1 (P1) and US2 (P1) are both P1, but US2's switcher row (T021) needs at least the
    *possibility* of a second instance to be meaningful to test — implement US1 first.
  - US3 (P2) depends on US2's T021 (the switcher row each close action, T025, attaches to).
  - US4 (P3) depends on US3's phase being in place conceptually (independent restart is most
    useful once instances are individually closeable/switchable), and directly on US2's T021 for
    its own per-entry restart affordance (T029).
- **Polish (Phase 7)**: Depends on all four user stories being complete.

### User Story Dependencies

- **US1 (P1)**: Foundational only.
- **US2 (P1)**: Foundational + benefits from US1 existing (nothing to switch between with only
  one instance), though its own tasks (T020/T021) have no hard code dependency on US1's tasks.
- **US3 (P2)**: Foundational + US2's T021 (the switcher row T025's close action attaches to).
- **US4 (P3)**: Foundational + US2's T021 (T029's per-entry restart affordance attaches to the
  same row).

### Parallel Opportunities

- T001 (icon) can run alongside all of Phase 2.
- Within Foundational: T003 (the one test file) has no parallel sibling test file this round
  (unlike feature 010's four parallel test files) — write it first, alone. T004 depends on
  nothing else and could start before T003 is finished, but write the failing test first per
  Principle I. T006/T007 both depend on T005 and touch different files (`app.rs` vs
  `ui/terminal.rs`) — parallelizable. T008/T009/T010/T011/T012 all touch `src/main.rs` — run
  sequentially, not in parallel, despite some having no direct dependency on each other's output.
- T013 (US1's keymap test) is independent of T003 (different file) — can run in parallel with
  any remaining Foundational work once Foundational's own tests (T003) are already written.
- T019, T022, T026, T030 (docs) can each run in parallel with the next story's work once their
  own story's implementation tasks are done (all edit the same
  `docs/user-guide/worktrees-and-sessions.md` file, so run them one after another in story order,
  not concurrently with each other).

---

## Parallel Example: Foundational → US1 handoff

```bash
# Foundational: write the one new pure test file first
Task: "Write failing pure tests in tests/session_shell_instances.rs"

# Once Foundational is green, US1's keymap test can start immediately (different file):
Task: "Write failing pure tests in tests/keymap.rs (is_new_terminal_chord)"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup (icon).
2. Complete Phase 2: Foundational (CRITICAL — blocks everything).
3. Complete Phase 3: User Story 1.
4. **STOP and VALIDATE**: `quickstart.md` Scenario 1 — a second instance opens independently and
   runs commands without disturbing the first.
5. Demo if ready — this alone delivers the feature's headline value (more than one shell per
   session).

### Incremental Delivery

1. Setup + Foundational → multi-instance infrastructure ready, no user-visible change yet.
2. Add US1 → open additional instances → validate Scenario 1 (MVP!).
3. Add US2 → switcher row, see/switch → validate Scenario 2.
4. Add US3 → close individually, with fallback rules → validate Scenario 3.
5. Add US4 → independent per-instance lifecycle/restart → validate Scenario 4.
6. Polish → cross-platform verification + full quickstart pass.

---

## Notes

- [P] tasks = different files, no dependencies.
- [Story] label maps task to specific user story for traceability.
- Several tasks are explicitly marked as having "no practical automated test" for real-PTY-process
  glue — this mirrors established precedent in this codebase (feature 010's `T018`/`T023`), not a
  new exception. Every genuinely pure decision (`ShellInstanceId` allocation, the `Session`
  instance mutators, the new keymap chord) IS tested first per Constitution Principle I.
- Foundational deliberately re-implements today's *existing* single-instance toggle/restart/
  exit-detection behavior against the new data shape (T009–T011) rather than deferring that to a
  user story — this is what keeps the Foundational checkpoint's "no user-visible change yet" bar
  true, and is what lets each of US1–US4 add one new *capability* on top of an already-working
  baseline instead of also having to fix a regression.
- Commit after each task or logical group; verify tests fail before implementing, then pass after.

---

## Phase 8: Convergence

**Purpose**: Close a gap between `spec.md`/`plan.md`/`tasks.md` intent and the current
implementation, found by `/speckit-converge` after Phase 7 was completed.

- [X] T034 Fix the spawn-failure recovery gap in `Message::ShellInstanceRestartRequested`'s
  handler (`src/main.rs`): it calls `session.restart_shell_instance(shell_id)` (advancing that
  instance's `ShellLifecycle` to `Starting`) before attempting `spawn_shell_pty`, with no
  rollback if the spawn fails — permanently hiding that instance's own restart affordance (shown
  only for `NotStarted`/`Exited`) since nothing ever moves it out of `Starting` again. Align it
  with `ensure_attached_process`'s already-correct `Regular` branch (spawn first, only transition
  state on success), or add an explicit `session.mark_shell_exited(shell_id)` in the `Err` arm,
  so a failed restart leaves the instance recoverable via the same button per FR-010 (partial).
  **Fixed**: removed the eager `restart_shell_instance` pre-call entirely — the handler now
  spawns first and only touches pure state via `Message::ShellInstanceRunning` on success,
  exactly matching `ensure_attached_process`'s pattern; on failure the instance's lifecycle is
  simply untouched (still `NotStarted`/`Exited`), so its restart affordance stays visible and the
  same button can be pressed again. Verified: `cargo fmt --check`, both build configs, both test
  suites (0 failures), and `cargo clippy --all-targets` (both configs) all clean.
  **Follow-up hardening** (same fix pass): `ensure_attached_process`'s `Regular` branch was
  additionally guarded so it only ever spawns for an `active_shell` instance that is still
  `Starting` (freshly opened, never yet spawned) — previously it would spawn for *any*
  `active_shell` id, which meant toggling modes (or `close_shell`'s FR-012 fallback reassigning
  `active_shell` to an already-`Exited` sibling) could silently auto-respawn an exited instance,
  contradicting FR-008's "no automatic restart on unexpected exit". An already-`Exited` instance
  now only restarts via the explicit `Message::ShellInstanceRestartRequested` path. Also added a
  `still_open` guard to `ShellInstanceRestartRequested`'s own handler so a restart request for an
  id that was closed in the meantime is a no-op rather than resurrecting a removed instance.
  Deduplicated the repeated spawn-success/failure block across all three call sites into
  `spawn_and_register_shell_instance`. Re-verified clean after this follow-up: `cargo fmt --check`,
  both build configs, both test suites, `cargo clippy --all-targets` (both configs).

---

## Phase 9: Bugfix BUG-001 — tab strip, close-control legibility, release-focus removal

**Bugfix**: 2026-08-14 — BUG-001 Updated from bugfix patch.

**Purpose**: `bugs/BUG-001.md`. The switcher does not read as tabs (only the active entry has a
container; inside a tab the label and close sit adjacent rather than centred/trailing), its close
control is near-invisible on the active tab, and the bar's release-focus button is obsolete.
All four defects live in one bar (`src/ui/terminal.rs`), so they are fixed as one pass.

**Requirements**: FR-004a, FR-011a, SC-007, SC-008 (this feature);
`023-terminal-focus-flow` FR-021b. Contracts: `contracts/terminal-instance-switcher-ui.md`,
`../006-real-terminal-emulator/contracts/focus-model.md`.

**No task is reopened.** T021, T025 and `023`'s T013 were each correct against their own text —
the text did not cover the visual form. This is a spec gap plus one superseded requirement, not
implementation drift.

### Tests for BUG-001 (MANDATORY — Constitution Principle I) ⚠️

- [X] T035 [P] Extend `crates/micold-client/tests/icon_roles.rs` to cover a glyph nested inside a
  filled container: assert `icon_role(IconSurface::PrimaryButton, roles)` has AA contrast against
  `roles.primary` (the fill `style::filled` paints) in **both** schemes, and — as the regression
  the bug actually was — that `icon_role(IconSurface::AppBarAction, roles)` (`on_surface`, the
  `IconButton` default) does **not**, so the wrong pairing is a failing assertion rather than a
  judgement call (FR-011a, SC-007). The existing file already has the WCAG helpers; this is a new
  case in it, not a new file. Pure — runs under `cargo test --no-default-features`.
- [X] T036 [P] Add a source gate in the new file
  `crates/micold-client/tests/terminal_tabs.rs` for the tab form (FR-004a). **Deviation, and the
  reason**: the gate as specified — scan `instance_switcher_row` and fail on `ButtonVariant::Text` —
  was written and *did* fail against correct code, because a tab legitimately **contains** a
  `Text`-variant button (the per-instance restart affordance) and a body scan cannot tell a nested
  control's variant from the tab's own. The rule was extracted instead into a pure
  `tab_variant(is_active)` in `src/ui/terminal.rs`, whose inline tests assert both arms draw a
  container and that the two differ (SC-004) — a value test rather than a text pattern. The source
  gate narrowed to `the_tab_variant_comes_from_the_shared_rule`, checking only that the call site
  still delegates, so the rule cannot be bypassed by choosing a variant inline again. Shape follows
  `crates/micold-client/tests/terminal_bar_stability.rs`.
- [X] T037 [P] Add `bar_has_no_release_focus_control` to
  `crates/micold-client/tests/terminal_bar_stability.rs`: fail if `src/ui/terminal.rs` still
  mentions `Icon::ReleaseFocus` (`023` FR-021b). Put it beside the existing
  `bar_does_not_branch_on_focus`, which must keep passing — the removal is unconditional precisely
  so that gate stays green.

### Implementation for BUG-001

- [X] T038 Remove the release-focus `IconButton` and its `Tooltip` from `pane()`'s bottom bar
  (`crates/micold-client/src/ui/terminal.rs`), together with the comment block explaining why it
  was pushed unconditionally. Delete nothing else: `Message::TerminalFocusReleased`,
  `release_terminal()`, and the reserved `Ctrl+Shift+E` / `Cmd+Shift+E` chord all stay — the chord
  is now the only explicit release (`023` FR-021b, `006` FR-011). `Icon::ReleaseFocus` stays in
  the shared vocabulary unless `tests/icons.rs` shows no remaining user; if it has none, remove it
  there too and update that test's pinned codepoint table and `Icon::ALL.len()`. (Depends on T037;
  supersedes `023`'s T013.)
- [X] T039 Rebuild each switcher entry in `instance_switcher_row`
  (`crates/micold-client/src/ui/terminal.rs`) as a tab (depends on T036; FR-004a, SC-008):
  wrap **every** entry, active and inactive alike, in the `Button` it already uses, keeping
  `ButtonVariant::Filled` for the active one and giving the inactive ones a low-emphasis
  *container* variant instead of `Text`; lay the content out as centred label + trailing close —
  replace `row![label, close].spacing(XS)` with a row that puts a `Space::new().width(Fill)`
  between them and centres the label in its own leading space, so the label sits at the tab's
  centre and the close sits at its right edge; and give the label a minimum width so neither a
  wider id nor a change of active tab reflows the row.
- [X] T040 Tint the controls nested inside a tab from the tab's own foreground rather than the
  `IconButton` default (depends on T035, T039; FR-011a, SC-007): pass
  the tab's own foreground instead of letting it fall through to `on_surface`
  (`src/ui/material/icon_button.rs`). **Deviation**: specified as
  `.tint(icon_role(IconSurface::PrimaryButton, r))`, which only covers the *active* tab —
  `icon_role` has no surface whose foreground is `primary`, the colour an `Outlined` inactive tab
  draws its label in. Both tabs now take `variant.content(r)` (made `pub(crate)`), the colour that
  variant draws its own label in: identical to `on_primary` for the filled case, correct for the
  outlined one, and automatically right for any variant added later. The per-entry restart affordance
  in the same tab took the same treatment — it is `Text`-variant and had the identical problem on a
  filled tab.
- [X] T041 [P] Update `docs/user-guide/worktrees-and-sessions.md` (Principle VII): it currently
  documents the release-focus affordance as one of the ways out of the terminal — leave the
  reserved chord and remove the affordance, matching `023`'s T032 cross-cutting rule that nothing
  in `docs/` may describe a release mechanism that no longer exists.
- [X] T043 [P] Patch `../023-terminal-focus-flow/quickstart.md`, which still treats the release
  affordance as a live control and would make T042's §B3 re-run unexecutable (FR-021b): in **§B2**
  delete the paragraph beginning "The release affordance is always in the bar, greyed when the
  terminal does not hold the keyboard" — §B2's own subject, the focus ring never blinking, is
  unaffected and stays; in **§B3.4** drop "affordance" from the release-method list, leaving
  "chord, press back into the pane". Do not touch `visual-pass.md` or `visual-pass-baseline.md` —
  those are records of passes already run, not instructions. While there, point the test-map row
  that reads "a regression would look like FR-021 breaking" at FR-021b as well, since FR-021 is now
  half superseded.
- [X] T044 [P] Add "### 8. The switcher reads as a tab strip — FR-004a, FR-011a, SC-007, SC-008
  (BUG-001)" to `quickstart.md`'s manual GUI section, giving T042 something to pass *against*.
  Scenario 2 covers SC-004 behaviourally (which instance is active) and stops there; the visual
  requirements have no written standard, which is how BUG-001 shipped. With two or more instances
  open, check: every tab sits in a container of the same shape and size, active and inactive alike
  — no entry is bare text; each tab's label is horizontally centred and its close control sits at
  the tab's right edge; selecting a different tab changes only the emphasis, leaving every tab's
  position and size untouched (nothing moves under the pointer); the close control is clearly
  visible on the **active**, highlighted tab, not just the inactive ones. Run all four in **both**
  the light and the dark theme.
- [X] T042 Run the visual pass with the `visual-pass` skill and record it: `quickstart.md`'s new
  §8 (T044) in **both** themes, plus `../023-terminal-focus-flow/visual-pass.md` §B3 re-run — as
  amended by T043 — to confirm focus behaviour is unchanged with the button gone. Append the §B3
  result to `../023-terminal-focus-flow/visual-pass.md` and the §8 result to a new
  `visual-pass.md` here. This is the class of defect the geometry gates cannot see — it is what let
  BUG-001 ship (depends on T038–T041, T043, T044).

**Checkpoint**: the switcher reads as a tab strip in both themes, every tab's close control is
legible including on the active tab, activation does not reflow the row, and the bar no longer
carries a release-focus button while `Ctrl+Shift+E` still releases.

### Phase 9 execution order

T035–T037 (gates, all [P], written failing first per Principle I) → T038 (needs T037), T039 (needs
T036) → T040 (needs T035, T039). T041, T043, T044 are docs/quickstart edits with no code
dependency and can run any time after their subject is settled; T043 and T044 must land **before**
T042, which executes what they specify. T041 and T043 both edit prose about the same removed
control — do them together to keep the wording consistent.

---

## Phase 10: Bugfix BUG-002 — the tabs are an indicator strip, not containers

**Bugfix**: 2026-08-16 — BUG-002 Updated from bugfix patch.

**Purpose**: `bugs/BUG-002.md`. "Tab" meant a Material **primary tab** — bare label plus an active
indicator — not the container-per-entry strip BUG-001 specified. The indicator sits at the tab's
**top** edge, because this bar is anchored to the window's bottom and the pane a tab selects is
above it.

**Requirements**: FR-004b, SC-009 (new); FR-004a's container clauses struck, its layout clauses and
FR-011a unchanged. Contract: "Tab form", "Active entry", "Active indicator".

**Two of BUG-001's gates encode the superseded rule and are replaced, not deleted.** A test that
pins a decision *should* fail when the decision changes — that is it working. What would be wrong is
deleting it and leaving the new rule unpinned, so each is replaced by its indicator equivalent.

### Tests for BUG-002 (MANDATORY — Constitution Principle I) ⚠️

- [X] T050 Replace `tab_variant_always_draws_a_container` and `tab_variant_distinguishes_the_active_tab`
  (inline `mod tests` in `src/ui/terminal.rs`) with the indicator rule: a pure
  `tab_indicator_colour(is_active, r) -> Option<Rgb>` returning `Some(accent)` for the active tab and
  `None` otherwise, tested over both arms — exactly one tab in a row can carry an indicator, and the
  active one always does (FR-004b, SC-004). Keep the shape of the tests being replaced; only the
  decision they pin changes.
- [X] T051 Replace `the_tab_variant_comes_from_the_shared_rule` in `tests/terminal_tabs.rs` with
  `the_active_tab_is_marked_by_an_indicator`: read `instance_switcher_row` and fail if it does not
  reserve the indicator's height for **every** tab (SC-008 — an indicator that appears on activation
  would push the row) and if the active/inactive choice does not come from `tab_indicator_colour`.
  Keep `the_nested_close_control_is_tinted_from_its_tab` (FR-011a) — it matters more without a
  container, since the close glyph now follows the accent.

### Implementation for BUG-002

- [X] T052 Add `anatomy::tab` to `crates/micold-core/src/tokens/anatomy.rs` with the indicator's
  thickness — §7's tab indicator, not the 1dp `text_field::INDICATOR` hairline. The tokens module is
  where a figure like this belongs; naming it locally in `terminal.rs` is how the 24-vs-48 error in
  BUG-001 happened.
- [X] T053 Rewrite each entry in `instance_switcher_row` (`src/ui/terminal.rs`) as an indicator tab
  (depends on T050–T052; FR-004b): `ButtonVariant::Text` for every tab, no container; a
  `column![indicator, content]` where the indicator is a `Space`-height accent bar on the active tab
  and an equally tall transparent gap on the others; the active label tinted with the accent, the
  inactive ones muted. Strike the doc comment's "a background-color difference is legible at a
  glance, unlike a thin edge accent" — that is the rationale BUG-002 overturns, and the reason it is
  now safe is that the cue is carried twice, by indicator *and* label colour.
- [X] T054 Size the label to its content with a maximum and ellipsis instead of the two-digit
  `TAB_LABEL_WIDTH` box (BUG-002 "Related"): an instance is to become renameable from a right-click
  menu, and a fixed two-digit width would have to be undone that day. Keep the leading spacer
  balancing the close control so the label stays centred (FR-004a's surviving clause).
- [X] T055 [P] Reword `quickstart.md` §8, which was written for containers — "every tab sits in a
  container of the same shape and size" is now false by design. It becomes: no tab draws a
  container; exactly one carries a top-edge indicator; the active label takes the accent; nothing
  reflows on activation.
- [X] T056 Run the visual pass with the `visual-pass` skill against the reworded §8, both themes,
  and append to `visual-pass.md` (depends on T053–T055). Pin the binaries per the skill's build
  section — this is the pass that caught the 12dp centring error last time, and the same class of
  error is live again here since the layout is being rebuilt. **Done, and the warning was right**:
  the first build had the active tab several times wider than the rest, because `Divider`'s
  `Length::Fill` resolved against the button's available space rather than the label's. The
  reserved-height gate passed throughout — the height was always correct; the defect was width.
  Fixed with a uniform `TAB_WIDTH`, and the pass then confirmed both themes, no reflow at identical
  crop geometry, and the squint test.

- [X] T063 [P] *(filed and completed as **T057**; renumbered by BUG-004's patch — Phase 11's
  T057–T062 were written in a parallel worktree against the same highest id and merged two days
  earlier, so two different tasks carried T057 in one file. The contiguous BUG-003 block keeps
  its numbers because its bug report, its commits and its checkpoint all name them; this one
  moves. Commit `fcc2f2a`, PR #194 and the comments in `tests/support/covered_states.rs` say
  T057 and mean this task.)* **Follow-up, not required by BUG-002**: register a covered state with two or more
  Regular Terminal instances in `019-layout-snapshot-parity`'s set, so the tab strip's geometry is
  under the snapshot gate at all. Discovered by this bugfix: the strip was rebuilt — containers to
  indicator, fixed tab width, an extra row per tab — and `layout_snapshot.txt` did not change one
  byte, because `session-terminal-bottom-bar` renders with at most one instance and the switcher
  returns `None` below two. Both defects the visual passes caught (a 12dp centring error, a tab
  several times too wide) are *pure geometry* and would have been in range of the fixture had the
  control ever been rendered into it. One registration by 019's FR-016; belongs to 019's covered
  set, and churns its fixture, which is why it is not done here.

  **Done, and it found a defect on its first regeneration.** Registered as
  `session-terminal-instance-tabs` in `crates/micold-client/tests/support/covered_states.rs` — one
  entry, per 019 FR-016 — with three instances, the **middle** one active (an active *trailing* tab
  cannot tell "the indicator spans its own tab" from "the indicator spans everything after the tab
  before it") and the trailing one `Exited`, so one tab draws the per-instance restart affordance
  its siblings do not. Eight anchors, so a failure names the strip, each tab, the active indicator
  and the exited tab's restart control rather than printing a path.

  What it recorded of BUG-002's own work is green: all three tabs measure **128.0 × 40.0**, the
  active tab's indicator is **112.0 × 3.0**, and the inactive tabs reserve a 3.0-high rule they do
  not draw. FR-004c and SC-008 hold, now against a fixture rather than against a screenshot.

  **The defect is the tab that restarts.** `TAB_WIDTH` gives the content row 112dp; its children
  want `48 (leading spacer) + 4 + 6.8 (label) + 4 + 48 (close) + 4 + 51.5 (restart) = 166.3`, so
  iced shrinks the trailing two — the restart button lays out **0.0 wide** and the close control at
  **45.2**, under `anatomy::button::MIN_TOUCH_TARGET`. A background instance that exits cannot be
  restarted from its own tab (feature 011 FR-010), and `ui/terminal.rs`'s comment on that affordance
  — "It widens its own tab, which SC-008 permits" — describes behaviour a fixed width forbids. The
  contradiction is between the comment and `TAB_WIDTH`, both written by BUG-002 and neither visible
  to any test until this state existed.

  Pinned as the fixture's baseline rather than fixed here, on 019 spec.md's own precedent — a
  snapshot records what it is shown, so a pre-existing defect becomes the expected value and the
  gate's contribution is to prove the fix. **`mise run test`: 1914 passed, 0 failed**, every gate
  green over a zero-width button, which measures how far outside their reach this control was.
  Needs its own bug report against feature 012: either `TAB_WIDTH` grows to fit a restartable tab,
  or the restart affordance moves out of the tab (a context menu is the obvious home, and the
  deferred rename below wants one anyway).

  **Cost, per 019 SC-006a**: the twelfth state adds **0.01s** — `layout_snapshot` 0.26s with it
  against 0.25s without, three runs each — against a 3s ceiling, with the suite at 27.0s against a
  60s budget. Measured warm by removing the state and putting it back, not derived. Recorded in
  `019-layout-snapshot-parity/quickstart.md` and `docs/development/layout-snapshot.md` with the
  thing worth flagging: the eleventh state cost 2.09s and `layout_snapshot` took 17.0s two weeks
  ago, and nothing committed to the apparatus since accounts for the difference. Both documents now
  say to re-measure rather than trust either figure.

**Checkpoint**: the strip reads as a tab bar with the active tab underlined from above, no
containers, and activation moves colour only.

### Deferred — recorded so it is not lost

Renaming an instance from a right-click menu, so a tab shows a name rather than an ordinal, is a
**feature and not part of BUG-002**. It is named here because it constrained T054, and because the
pieces already exist: `ContextMenu` is imported by `terminal.rs` today for the terminal pane's own
menu, and `ShellInstance` would need a `title: Option<String>` beside its `lifecycle`. Worth its own
spec rather than an ad-hoc addition — it touches persistence (does a name survive a restart, given
FR-017 restores at most one instance?) and the daemon's session state, neither of which this bug
should decide.
---

## Phase 11: Bugfix BUG-003 — an instance's lifecycle never leaves `Starting`

**Goal**: Make FR-008 true. Three of its four states were unreachable in production: an instance was
set `Starting` by `open_shell_instance` and never moved again, so a live shell read `starting…` for
its whole life and an exited one read the same.

The transitions existed and were tested. `mark_shell_running`/`mark_shell_exited` were reachable only
from `Message::ShellInstanceRunning`/`ShellInstanceExited`, which nothing in the client emits — and
there was nothing to emit them *from*, because the daemon modelled shell instances in its live
registry and not on the wire at all. Same shape as `010` BUG-011, one layer out.

### Tests for BUG-003 (MANDATORY — Constitution Principle I) ⚠️

- [X] T057 [BUG-003] Failing test in `crates/micold-daemon/tests/shell_instances.rs`: the catalog
  snapshot reports which of a session's shell instances are live — none for a session the daemon is
  not hosting, none for a live `Primary` (the field names instances, not processes), both ids after
  two opens, and one after a close. The close case is the point: it is the only signal that makes
  `exited` reachable, since a client watching for frames cannot tell a dead shell from a quiet one
- [X] T058 [BUG-003] Failing test in `crates/micold-client/src/shell/daemon_sync.rs`'s test module:
  `reconcile_catalog` marks an instance `Running` when the snapshot lists it and `Exited` when it
  stops being listed; leaves an instance never seen live alone; and creates nothing for an id the
  client does not have. Asserted through the snapshot rather than by driving the messages —
  `tests/app_state.rs` already drives those, and proving the transitions correct is precisely what
  failed to notice nothing invoked them. **This test corrected the bug report**: it was written
  expecting `NotStarted` and found `Starting`, because `open_shell_instance` calls `start_shell()`

### Implementation for BUG-003

- [X] T059 [BUG-003] Add `live_shells: Vec<ShellInstanceId>` to `SessionSummary`
  (`crates/micold-core/src/protocol/messages.rs`) and bump `PROTOCOL_VERSION` 5 → 6 — wire-visible,
  so the integer moves with it (`010` FR-021). The doc comment carries the asymmetry that matters:
  an id present means the process exists; an id absent means "not hosting", which covers a spawn in
  flight as well as an exit
- [X] T060 [BUG-003] Fill it in `DaemonState::overlay_live_summaries`
  (`crates/micold-daemon/src/state.rs`) from `LiveSession.procs`' `SessionProcess::Shell` keys, beside
  the `activity`/`input_serial`/title overlays already there and for the same reason; the durable
  projection in `catalog.rs` defaults it empty
- [X] T061 [BUG-003] Adopt it in `reconcile_catalog`
  (`crates/micold-client/src/shell/daemon_sync.rs`), beside every other value the daemon publishes.
  Absence is read as death **only** for an instance already seen `Running`: a spawn is in flight for
  a tick or two after `SessionOpenShell`, and treating that as an exit would flap the bar for every
  terminal opened
- [X] T062 [BUG-003] Correct the two comments in `crates/micold-client/src/app.rs` claiming the binary
  "follows up with `ShellInstanceRunning` once it's up", and document both variants as emitted
  nowhere. **Kept rather than deleted**, unlike the temptation: `shell/daemon_sync.rs` lives in the
  binary crate, so `reconcile_catalog` is unreachable from `tests/`, and these variants are the only
  lever the integration tests have — deleting them would delete that coverage, including feature
  023's FR-019 rule that a session reaching `Running` must not move the keyboard

**Checkpoint**: a Regular Terminal reads `running` while its shell is up, `exited` once it is not,
and FR-010's per-instance restart control appears for the instance that needs it.

**Bugfix**: 2026-08-16 — BUG-003. **No requirement added**: FR-008 already stated this exactly, and
no task is reopened — the client-side machinery T029/T030 built is correct and was simply never
driven. `PROTOCOL_VERSION` 5 → 6 is the visible cost; a client and service across that boundary refuse
each other and the service must be restarted once (`010` FR-021/FR-022). See `bugs/BUG-003.md`.

---

## Phase 12: Bugfix BUG-004 — the tab that offers a restart cannot fit one

**Goal**: Make FR-010 reachable from the tab strip, and make the class of defect that hid it
reportable.

Phase 11's checkpoint reads "FR-010's per-instance restart control **appears** for the instance that
needs it", and it does — at 0.0dp wide. `TAB_WIDTH` gives a tab's content row 112dp and a restartable
tab's children ask for 166.3, so iced settles the 54.3dp shortfall by shrinking the trailing two: the
restart button vanishes and the close control beside it drops to 45.2, under the 48dp target feature
018 FR-027 sets. Nothing overflows, nothing escapes, and `mise run test` is green over all of it.

The width is the fix. The gate is the point: a squeezed child satisfies every invariant this
repository currently checks, which is why a control could be reduced to nothing between two bugfixes
that were each looking straight at it.

### Tests for BUG-004 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T064 [BUG-004] Failing test in `crates/micold-client/tests/` (new `tab_children_fit.rs`, or a
  gate compiled into the `layout_snapshot` binary beside `sibling_parity` if it needs the shared
  record cache): in the `session-terminal-instance-tabs` covered state, **no child of a tab is laid
  out narrower than the width it asks for** (SC-010). Ask it the way the defect presents: every
  interactive control in a tab must measure at least `anatomy::button::MIN_TOUCH_TARGET` wide, and
  the sum of a tab's children plus its gaps and padding must not exceed the tab. Must **fail on
  today's `main`**, naming the exited tab's restart control at 0.0 and its close at 45.2. This is the
  gate the whole phase exists for, and it outlives the particular fix: it is the first check here
  that reads a laid-out child against *what it requested* rather than against a constant or against
  its parent's bounds, so it holds whether the affordance is inside the tab or not — feature 018's
  BUG-002 (a 48dp figure written and then overwritten) and this bug (the same figure competed away)
  would both have failed it
- [ ] T065 [P] [BUG-004] Failing value test beside `tab_indicator_colour` in `src/ui/terminal.rs`'s
  `mod tests`: `TAB_WIDTH` equals the sum its derivation requires (FR-004c), computed from
  `anatomy::button::MIN_TOUCH_TARGET`, `spacing::SM`, `spacing::XS` and a minimum label rather than
  written as a literal. A test that re-states a magic number proves nothing; this one fails if any
  constant it is built from moves, which is the property FR-004c requires and a chosen figure cannot
  have. Note that the figure does **not** change in this bugfix — 128dp is what the derivation gives
  once FR-010b takes the restart affordance out — so this task is pure regression cover: it is the
  test that would have failed the day T056 chose the number
- [ ] T066 [BUG-004] Failing test for the secondary-click primitive, in a `mod tests` beside it: a
  right (secondary) press inside the wrapped content publishes the message with the press point, a
  press outside publishes nothing, and a **primary** press publishes nothing and is left for the
  child — the tab's own `on_press` selects the instance and must keep working through the wrapper
- [ ] T067 [P] [BUG-004] Failing test in `crates/micold-client/tests/app_state.rs` (or beside the
  reducer): opening the tab menu for one instance records that instance; opening it for another
  replaces rather than stacks; the "close every menu" path clears it; and restart dispatched from the
  menu targets the instance the menu was opened on, **not** the active one (FR-010a, FR-010b)

### Implementation for BUG-004

- [ ] T068 [BUG-004] Add the secondary-click primitive to `crates/micold-client/src/ui/cdk/` (new
  `context_area.rs`, declared in `cdk/mod.rs`) — a single-child wrapper that delegates layout, draw,
  operate and overlay to its content and intercepts only `mouse::Event::ButtonPressed(Right)` while
  the cursor is over it, publishing a message built from the press point (depends on T066). It
  belongs in the **cdk** and not in `material/`: it holds no appearance, which is the boundary
  `tests/material_boundary.rs` enforces. `ui/material/checkbox.rs::TakesTheKeyboard` is the
  delegation template; `ui/material/terminal_pane.rs` is the existing right-click handler and the
  reason this is a *new* primitive rather than a reused one — that one is fused into a bespoke widget
  and cannot wrap anything
- [ ] T069 [BUG-004] Wire the menu's state and messages:
  `Message::ShellInstanceMenuRequested(SessionId, ShellInstanceId, u16, u16)` and
  `ShellInstanceMenuClosed` in `src/app.rs`, with `shell_instance_menu: Option<(ShellInstanceId,
  u16, u16)>` on `State` (depends on T067). Clear it wherever the other menus are cleared — the list
  at `app.rs`'s "close every menu" path names `worktree_menu_open`, `session_menu_open` and
  `terminal_context_menu`, and a fourth that is not in it is a menu that survives a navigation.
  Register the surface beside `terminal_context_menu` in `src/features/session.rs` and
  `src/overlay/registry.rs`
- [ ] T070 [BUG-004] Remove the restart affordance from `instance_switcher_row`
  (`src/ui/terminal.rs`) and offer it from a `ContextMenu` on the tab instead (depends on
  T068–T069; FR-010b): wrap each tab in the T068 primitive, and mount the menu on the bar the way
  `pane()` already mounts the terminal's own context menu — `cdk::overlay::Overlay` around the
  content, anchored at the press point. Items: **Restart** when that instance's own lifecycle is
  `NotStarted | Exited`, **Close** always. Not Rename: an instance has no title to set, and giving it
  one touches persistence and the daemon's session state, which is the separate feature the
  "Deferred" note below already describes. With the affordance gone the tab's children are the
  leading spacer, the label and the close control again, which is exactly what `TAB_WIDTH` was
  derived for, and T064 goes green without the figure moving
- [ ] T071 [BUG-004] Correct the comment in `src/ui/terminal.rs` on the restart affordance — "It
  widens its own tab, which SC-008 permits: that is a lifecycle change, not a change of which tab is
  active." That was true before T056 introduced a fixed width and false from that commit on, in the
  same file, and no test reads comments. It should now say why the affordance is not in the tab at
  all
- [ ] T072 [BUG-004] Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` with
  `UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot` (depends on T070).
  The diff is the artefact: `terminal.tabs.exited.restart` disappears, the exited tab's close returns
  to 48.0, and the three tabs and the strip do **not** move — a tab strip that changed width here
  would mean the derivation was wrong in the other direction. Every other covered state must be
  untouched; the strip is in one state only. The `terminal.tabs.exited.restart` anchor in
  `tests/support/covered_states.rs` must go with it, or it fails by name, which is the behaviour that
  makes an anchor worth having
- [ ] T073 [P] [BUG-004] Re-run `quickstart.md` **§4 and §8** with the `visual-pass` skill and record
  both in `visual-pass.md` (depends on T070). §4 "Independent lifecycle and restart" is the section
  that would have caught this and it has not been run since `TAB_WIDTH` existed — BUG-002's pass ran
  §8, the appearance section, which was right about its own subject. §8 is here because the tab's
  children change: with the restart gone the label's centring is back on the tab's midline, and that
  is §8's subject and beyond the fixture's reach — a 12dp centring error is precisely what it missed
  last time. In §4, exercise a **background** exit specifically, through the new menu: the whole of
  FR-010a is about the instance that is not the active one
- [ ] T074 [P] [BUG-004] Update `contracts/terminal-instance-switcher-ui.md` — its "Tab form" section
  lists what sets the fixed width, and the per-entry restart bullet places the affordance inside the
  tab. Move it to the menu, record that the width is derived rather than chosen, and state what the
  tab's children now are
- [ ] T075 [P] [BUG-004] Document the tab context menu in `docs/user-guide/worktrees-and-sessions.md`
  (Principle VII): right-click a terminal tab to restart a stopped instance or close it. The
  affordance is no longer visible on the tab, so the user guide is now the only place it is written
  down for a user — which is the cost FR-010b accepts, and the mitigation

**Checkpoint**: an instance that exits in the background can be restarted from its own tab's menu
without being selected first, every tab is still 128dp with a full 48dp close target, and a gate
fails if any tab's child is ever squeezed again.

**Bugfix**: 2026-08-18 — BUG-004 Updated from bugfix patch. Phase 12 added (T064–T075). **No task
reopened**: T029 built the affordance correctly and its condition is still right, and T056 chose a
width that solved the defect its visual pass could see, against three tab states none of which was
exited. The conflict is between two requirements written for two different bugfixes — FR-004c
(BUG-002) and FR-011a (BUG-001) — and is invisible from either one alone. **Re-patched during
implementation** once the derivation was computed: FR-004c's own rule gives a 204dp tab and 628dp of
a 1014dp bar at three instances, so FR-010b moves the affordance out of the tab rather than widening
every tab for a child most never draw; the earlier T066/T067 (derive a wider width, re-balance the
leading spacer) are replaced by T068–T070, and `TAB_WIDTH` does not move. **One renumbering**: Phase
10's T057 becomes T063, because Phase 11's T057–T062 were written in a parallel worktree against the
same highest id; the note on T063 records what its commits and PR call it. See `bugs/BUG-004.md`.
