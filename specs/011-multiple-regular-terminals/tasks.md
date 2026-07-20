# Tasks: Multiple Regular Terminal Instances per Session

**Input**: Design documents from `/specs/011-multiple-regular-terminals/`

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
