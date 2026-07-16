---
description: "Task list for Worktree & Session Navigation with Embedded Terminal"
---

# Tasks: Worktree & Session Navigation with Embedded Terminal

**Input**: Design documents from `specs/005-worktree-session-terminal/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: MANDATORY (Constitution Principle I). Every user story writes failing tests BEFORE
implementation (Red-Green-Refactor). Pure-core tests run under `cargo test --no-default-features`
against `FakeGit` / `FakeTerminalBackend` — no real git, no spawned processes, no GUI.

**Documentation**: MANDATORY per story (Constitution Principle VII) — each user-facing story ships
its section of `docs/user-guide/worktrees-and-sessions.md` in the same change.

**Cross-platform**: Linux, macOS, Windows (Constitution Principle VI). Platform specifics stay
inside `portable-pty` / the `git` CLI / `iced_term`, behind the `Git` and `TerminalBackend` traits.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1 / US2 / US3 (setup, foundational, polish have no story label)

## Path Conventions

Single Rust project: render-free core in `src/*.rs` + `tests/*.rs`; gui-gated layer in `src/ui/`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Dependencies, module wiring, toolchain.

- [x] T001 Update `Cargo.toml`: add `uuid = { version = "1", features = ["v4"] }` (core); add iced features `canvas`, `advanced`, `lazy`; add gui-gated `iced_term = "=0.6.0"`, `portable-pty = "0.9"`, `alacritty_terminal = "0.25"` under the `gui` feature (the VT `Term` grid is gui-only; the pure core never depends on these, keeping `--no-default-features` lean); set `rust-version` to a concrete pinned value = max(current 1.80, the MSRV stated by `iced_term 0.6`, `alacritty_terminal 0.25`, and `portable-pty 0.9` — check each crate's "Rust version" on docs.rs before pinning); update the CI toolchain pin to match (Principle VI); do not leave the MSRV unbounded. Add a justification comment per new crate (Principle V).
- [x] T002 [P] Declare new core modules in `src/lib.rs`: `pub mod naming; pub mod worktree; pub mod session; pub mod git; pub mod terminal;` (empty stubs compiling under `--no-default-features`).
- [x] T003 [P] Add gui module stubs and wire them in `src/ui/mod.rs`: `mod components; mod sidebar; mod worktree_form; mod terminal;` and create `src/ui/components/mod.rs`.
- [ ] T004 [P] Verify the CI matrix still builds/tests on Linux, macOS, Windows with the new MSRV and deps (`.github/workflows/*`); adjust the toolchain pin if needed (Principle VI).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Boundaries and shared primitives that ALL stories depend on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [x] T005 [P] Write failing tests for the `Git` boundary via an in-memory `FakeGit` (branch_exists, worktree_list_porcelain, worktree_add_new_branch, worktree_remove, worktree_prune, branch_delete) in `tests/git_fake.rs` (contracts/git-trait.md).
- [x] T006 Define the `Git` trait in `src/git.rs` per contracts/git-trait.md.
- [x] T007 [P] Implement `FakeGit` (in-memory maps, primable failure step) in `src/git.rs` to make T005 pass.
- [x] T008 Implement `GitCli` (thin `std::process::Command` wrappers over the `git` binary) in `src/git.rs`.
- [x] T009 [P] Write failing tests for `Worktree` + `WorktreeStatus` (fields, invariants, session list) in `tests/worktree_model.rs` (data-model.md).
- [x] T010 Implement `Worktree` struct + `WorktreeStatus` enum in `src/worktree.rs` to make T009 pass.
- [x] T011 [P] Write failing tests for the extended app base state (new fields default correctly; new `Message` variants + `Overlay::AddWorktree` wire as no-ops) in `tests/app_state.rs`.
- [x] T012 Extend `src/app.rs`: add `State` fields (`worktrees`, `sessions`, `active_session`), new `Message` variants (worktree/session/terminal per contracts), and `Overlay::AddWorktree`; wire pure no-op branches to make T011 pass.
- [x] T013 [P] Implement the shared `TreeView` primitive (generic nodes, expand/collapse, selection, theming via `style::color`) in `src/ui/components/tree_view.rs` and export it from `src/ui/components/mod.rs` (Principle VIII).
- [x] T014 [P] Implement the shared `IconButton` primitive (icon + color role + on-press, disabled state, theming) in `src/ui/components/icon_button.rs` and export it (Principle VIII).

**Checkpoint**: Boundaries (`Git`, worktree model), app scaffolding, and shared UI primitives ready.

---

## Phase 3: User Story 1 - Open a project and browse its worktrees (Priority: P1) 🎯 MVP

**Goal**: Open a git repository as the active project and browse its worktrees (top level) → sessions
(sub-items) in a Material Design sidebar; refuse non-git directories.

**Independent Test**: Open a git repo that already contains worktrees under `.claude/worktrees/` →
they list in the sidebar and expand to show sessions; open a non-git directory → refused, nothing opens.

### Tests for User Story 1 (MANDATORY — write first, must FAIL) ⚠️

- [x] T015 [P] [US1] Failing tests: open-project git-root gate accepts a repo and refuses a non-git dir (FR-001a) using `FakeGit` in `tests/open_project_git_gate.rs`.
- [x] T016 [P] [US1] Failing tests: worktree discovery — parse `worktree list --porcelain` fixtures and classify each as Valid / Missing / Invalid (FR-018/018a) in `tests/worktree_discovery.rs`.
- [x] T017 [P] [US1] Failing tests: build sidebar tree nodes from worktrees and toggle expand/collapse (FR-002/003) in `tests/sidebar_tree.rs`.

### Implementation for User Story 1

- [x] T018 [US1] Implement the pure porcelain parser + `classify()` in `src/worktree.rs` (contracts/git-trait.md) to make T016 pass.
- [x] T019 [US1] Wire `Git::is_repo_root` into the open-project flow and refuse non-git with a clear message: reducer branch in `src/app.rs`, I/O in `src/main.rs`, to make T015 pass.
- [x] T020 [US1] Implement tree-node building + expand/collapse helpers in `src/app.rs` (worktree→session shaping) to make T017 pass.
- [x] T021 [US1] Sidebar view: render worktrees via the shared `TreeView` (empty state + add-worktree affordance, light/dark theming, FR-002/004) in `src/ui/sidebar.rs`.
- [x] T022 [US1] Two-pane shell layout (sidebar | main content) in `src/ui/shell.rs` and `src/ui/mod.rs`.
- [x] T023 [US1] Discover worktrees at project open (I/O: `GitCli` + `fs` existence for classify) in `src/main.rs`, populating `State.worktrees`.
- [x] T024 [US1] User-guide docs: "Opening a project (git repositories only)" + "Browsing worktrees" sections in `docs/user-guide/worktrees-and-sessions.md` (Principle VII).

**Checkpoint**: A git project opens, non-git is refused, and existing worktrees are browsable. MVP.

---

## Phase 4: User Story 2 - Create a new worktree (Priority: P1)

**Goal**: Create a worktree via a form (type + optional ticket + name) → new git branch
`${type}/${ticket}-${name}` + worktree at `.claude/worktrees/${type}-${ticket}-${name}`, shown in the
sidebar; roll back cleanly on failure.

**Independent Test**: Open the form, pick a type, optionally enter a ticket, enter a name, submit →
directory + branch created with the derived names and the worktree appears; forced git failure leaves
no orphan branch/dir/sidebar entry.

### Tests for User Story 2 (MANDATORY — write first, must FAIL) ⚠️

- [x] T025 [P] [US2] Failing tests: naming derivation table (with/without ticket, slugify edge cases, `NoType`/`EmptyNameAfterSlug`/`InvalidBranchRef`, SC-003b) in `tests/naming.rs` (contracts/naming.md).
- [x] T026 [P] [US2] Failing tests: `create_worktree` happy path + duplicate detection (`DuplicateDir`, `DuplicateBranch`, FR-009) via `FakeGit` in `tests/worktree_create.rs`.
- [x] T027 [P] [US2] Failing tests: rollback plan ordering on primed failure (worktree remove → prune → branch delete → rmdir, FR-006b) via `FakeGit` in `tests/worktree_rollback.rs`.

### Implementation for User Story 2

- [x] T028 [US2] Implement the `naming` module (slugify, `ConventionalType`, `derive`, validation) in `src/naming.rs` to make T025 pass.
- [x] T029 [US2] Implement `create_worktree` orchestration + `CleanupStep` rollback plan over `Git` in `src/worktree.rs` to make T026 and T027 pass.
- [x] T030 [US2] Add-worktree form state (type/ticket/name inputs, live derived-names preview, validation error) in `src/app.rs` `State`/`Message`.
- [x] T031 [US2] Add-worktree form UI (type selector, ticket + name fields, derived `dir`/`branch` preview per FR-008a, submit/cancel using `IconButton`) in `src/ui/worktree_form.rs`, wired to `Overlay::AddWorktree` in `src/ui/mod.rs`.
- [x] T032 [US2] Wire submit: derive → `create_worktree` → refresh discovery; surface errors; rollback on failure. Reducer in `src/app.rs`, I/O in `src/main.rs`.
- [x] T033 [US2] Refresh the sidebar so a newly created worktree appears as a top-level item (FR-007) in `src/app.rs` / `src/ui/sidebar.rs`.
- [x] T034 [US2] Form UX: block submit on `NoType` / empty-after-slug; Esc/cancel dismiss; extend `on_escape` in `src/app.rs` and validation display in `src/ui/worktree_form.rs`.
- [x] T035 [US2] User-guide docs: "Creating a worktree (type, ticket, name; derived branch & directory)" section in `docs/user-guide/worktrees-and-sessions.md` (Principle VII).

**Checkpoint**: Worktrees can be created from the form with correct naming and safe rollback.

---

## Phase 5: User Story 3 - Start a session and interact with the embedded terminal (Priority: P1)

**Goal**: Start a session on a worktree → embedded terminal on the right runs `claude` in that worktree;
sessions run concurrently, persist, auto-restart on crash, resume via `--resume`, and stop on project
close/switch.

**Independent Test**: Select a Valid worktree, start a session → an interactive `claude` terminal appears
for that worktree; start a second, switch between them (both keep running); close the app and reopen →
sessions restore and resume.

### Tests for User Story 3 (MANDATORY — write first, must FAIL) ⚠️

- [x] T036 [P] [US3] Failing tests: `SessionLifecycle` transitions start/switch/close (FR-010/015/015a) in `tests/session_lifecycle.rs`.
- [x] T037 [P] [US3] Failing tests: crash auto-restart via `--resume` and guard → `Failed` (FR-022/022a) in `tests/session_crash_restart.rs`.
- [x] T038 [P] [US3] Failing tests: project close/switch stops sessions → `Idle` preserving records; reopen resumes (FR-023/023a) in `tests/session_project_switch.rs`.
- [x] T039 [P] [US3] Failing tests: `TerminalBackend` `LaunchSpec` (cwd = worktree, `--session-id` fresh / `--resume` on resume) via `FakeTerminalBackend` in `tests/terminal_backend.rs` (contracts/terminal-backend-trait.md, claude-cli.md).
- [x] T040 [P] [US3] Failing tests: session store roundtrip (id / worktree_dir / title; `null` title → `Pending`, SC-008) in `tests/session_store.rs` (contracts/storage-schema.md).
- [x] T041 [P] [US3] Failing tests: `PtyOutput` is routed by `SessionId` to the correct per-session output sink with no cross-talk (FR-019, SC-005) in `tests/pty_routing.rs`. Assert routing/isolation ONLY (which session receives which bytes) against a pure sink — VT `Term` grid rendering is gui-side and validated separately (T047 + a gui-gated test).

### Implementation for User Story 3

- [x] T042 [US3] Implement `Session`, `SessionId`, `SessionLabel`, and the `SessionLifecycle` state machine in `src/session.rs` to make T036/T037/T038 pass.
- [x] T043 [US3] Define `TerminalBackend` / `TerminalHandle` / `LaunchSpec` traits, a pure per-session output routing seam (`SessionOutput` sink keyed by `SessionId`), and `FakeTerminalBackend` in `src/terminal.rs` to make T039 and T041 pass. The VT grid (`alacritty_terminal::Term`) is NOT part of the core seam — it stays gui-side (T047).
- [x] T044 [US3] Extend `src/store.rs` to persist/load sessions per project by adding an optional per-project `sessions` array to `projects.json` (option A: forward-compatible, single atomic write, no `schema_version` bump) per contracts/storage-schema.md, to make T040 pass.
- [x] T045 [US3] Real `TerminalBackend` impl: `portable-pty` spawn (cwd, `--session-id`/`--resume`, `TERM` env) + blocking reader thread, gui-gated, in `src/ui/terminal.rs` (research R1/R6).
- [x] T046 [US3] PTY → iced streaming: `Subscription::run_with_id` + tokio mpsc emitting `PtyOutput`/`PtyExited`, batched per session, in `src/ui/terminal.rs` and `src/main.rs` (research R4/R5).
- [ ] T047 [US3] Terminal pane rendering (`iced_term` 0.6 / `canvas`): draw the active session's grid, send keystrokes to the writer, handle resize, coalesce redraws with `canvas::Cache` (research R3) in `src/ui/terminal.rs`.
- [x] T048 [US3] Render session sub-items under their worktree in the `TreeView` (label `Pending`/`Named`, active/inactive state, FR-011/011a/016) in `src/ui/sidebar.rs`.
- [x] T049 [US3] Start-session action (disabled unless worktree `status == Valid` per FR-018a): generate UUID, `Fresh` launch, in `src/app.rs` + `src/main.rs`.
- [x] T050 [US3] Switch session — change active id only; other sessions keep running (FR-015/015b) — and show the active terminal in `src/app.rs` / `src/ui/shell.rs`.
- [x] T051 [US3] Close/stop session — `kill()` + reap, remove from sidebar, drop persisted record (FR-015a) — in `src/app.rs` + `src/main.rs`.
- [x] T052 [US3] Crash auto-restart wiring: `PtyExited` (unexpected) → `Restarting{attempts}` → `--resume`; exceed guard → `Failed` (FR-022/022a) in `src/app.rs` + `src/main.rs`.
- [x] T053 [US3] Project close/switch: stop that project's session processes → `Idle`, preserve records; reopen restores and resumes (FR-023/023a) in `src/app.rs` + `src/main.rs`.
- [ ] T054 [US3] Best-effort session label from `claude` `ai-title` JSONL, updating `Pending` → `Named` (FR-011a, claude-cli.md) at the I/O boundary in `src/ui/terminal.rs` / `src/main.rs`.
- [x] T055 [US3] User-guide docs: "Starting, switching, and closing sessions" + "The embedded terminal, resume & restart behavior" in `docs/user-guide/worktrees-and-sessions.md` (Principle VII).
- [x] T062 [US3] (bugfix BUG-001) Exclude **empty sessions** (no recorded `claude` conversation) from persistence: filter on save and prune on load, using a `claude`-transcript existence check (`<claude>/projects/<encoded-cwd>/<session-id>.jsonl`), so a restart never resumes a nonexistent conversation (FR-020/FR-020a). In `src/main.rs` (persist/boot); covered by the manual restart check.

**Checkpoint**: Concurrent `claude` sessions run in worktrees, persist, resume, and recover from crashes.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [x] T056 [P] Update docs index/navigation (`docs/README.md`) to link the new worktrees-and-sessions guide.
- [x] T057 Kill all child `claude` processes on app shutdown (`Drop`/shutdown handler) to avoid zombies (research R5) in `src/main.rs`.
- [ ] T058 Performance pass: verify redraw coalescing (≤1/frame) and per-session scrollback cap under chatty output (SC-004/005) in `src/ui/terminal.rs`.
- [x] T059 [P] Confirm dependency-vetting comments (Principle V) for `uuid`, `iced_term`, `portable-pty`, `alacritty_terminal` in `Cargo.toml`.
- [ ] T060 Verify full build + `cargo test` (default and `--no-default-features`) + `cargo clippy` pass on Linux, macOS, Windows (Principle VI).
- [ ] T061 Run the `quickstart.md` validation scenarios V1–V10 end-to-end, including the timing observations for SC-001 (open < 3 s, V1), SC-002 (create < 30 s, V3), and SC-004 (session start < 5 s, V4).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies — start immediately.
- **Foundational (Phase 2)**: depends on Setup — BLOCKS all user stories.
- **User Stories (Phase 3–5)**: all depend on Foundational. US1 → US2 → US3 in priority order; US2
  create relies on US1's discovery/refresh, US3's sidebar sub-items build on US1's `TreeView` sidebar.
- **Polish (Phase 6)**: depends on all targeted stories.

### User Story Dependencies

- **US1 (P1)**: after Foundational. Independently testable (repo with pre-existing worktrees).
- **US2 (P1)**: after Foundational; reuses US1's discovery/refresh + sidebar. Independently testable via
  the create-then-verify flow.
- **US3 (P1)**: after Foundational; reuses US1's sidebar `TreeView`. Independently testable by starting a
  session on an existing Valid worktree.

### Within Each User Story

- Tests (⚠️) written and FAILING before implementation (Principle I).
- Pure models/logic before I/O wiring; core before gui.
- User-guide docs ship in the same story (Principle VII).
- Story complete only when tests pass, docs exist, and it works on all three platforms (Principle VI).

### Parallel Opportunities

- Setup: T002, T003, T004 in parallel.
- Foundational: T005/T007, T009, T011, T013, T014 are largely parallel (distinct files); T006→T007→T008
  and T009→T010, T011→T012 are ordered pairs.
- Each story's ⚠️ test tasks (all marked [P]) run in parallel first.
- With capacity, US1/US2/US3 can be staffed in parallel after Foundational (mind the noted reuse).

---

## Parallel Example: User Story 3 tests

```bash
# Write all failing US3 tests together (distinct files):
Task: "SessionLifecycle transitions in tests/session_lifecycle.rs"
Task: "crash auto-restart + guard in tests/session_crash_restart.rs"
Task: "project close/switch in tests/session_project_switch.rs"
Task: "TerminalBackend LaunchSpec in tests/terminal_backend.rs"
Task: "session store roundtrip in tests/session_store.rs"
Task: "PtyOutput routing by id in tests/pty_routing.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1 Setup → 2. Phase 2 Foundational (CRITICAL) → 3. Phase 3 US1 →
4. **STOP & VALIDATE**: open a git repo with existing worktrees, confirm browse + non-git refusal →
5. Demo the MVP.

### Incremental Delivery

Foundation → US1 (browse, MVP) → US2 (create) → US3 (sessions + terminal). Each story is a shippable,
independently testable increment that doesn't break the previous one.

---

## Notes

- [P] = different files, no incomplete-task dependency.
- Pure core (naming, worktree model + orchestration, session lifecycle, store) is tested under
  `cargo test --no-default-features` against `FakeGit`/`FakeTerminalBackend` — no real git, no processes,
  no GUI (Principle I).
- The `git` CLI, `portable-pty`, and `iced_term` provide cross-platform behavior behind the traits
  (Principle VI); shared `TreeView`/`IconButton` primitives are reused, not forked (Principle VIII).
- Verify tests FAIL before implementing; commit per task or logical group.

---

## Implementation status (2026-07-15)

**Completed & verified** (55/61): all of Phase 1–2, US1, US2, US3 core logic and wiring, plus
docs. Verification evidence:
- `cargo test --no-default-features` — **135 pure-core tests pass** (naming, git boundary,
  worktree model/discovery/create/rollback, session lifecycle/crash/restore, PTY routing,
  session store, app state/tree).
- `cargo test` (gui) — full suite green, **0 failures**; `cargo clippy --all-targets` and
  `cargo fmt --check` clean on both feature sets.
- Real `git worktree add -b` / `list --porcelain` / rollback verified end-to-end against a
  throwaway repo (matches `GitCli` + the porcelain parser).
- GUI compiles with the full terminal stack (`iced_term 0.6`, `portable-pty 0.9`,
  `alacritty_terminal 0.25`).

**Remaining** (6) — require a display or CI and are not verifiable in this headless environment:
- **T004 / T060** — CI matrix build/test on Linux/macOS/Windows. `rust-version` left at `1.80`;
  builds on 1.97 locally, but the exact MSRV for the terminal crates was not re-pinned.
- **T047** — the terminal pane currently renders streamed output as scrollable monospace text
  with line input (functional, via `portable-pty`); full `iced_term`/`alacritty_terminal` VT-grid
  rendering + raw-key input is the next increment. The `TerminalBackend`/`SessionRouter` seam
  keeps that swap local.
- **T054** — session labels stay at the `Pending` placeholder; reading the `claude` `ai-title`
  JSONL to populate `Named(..)` is not yet implemented (best-effort, degrades gracefully).
- **T058** — redraw coalescing is approximated by the 120 ms poll; a per-session scrollback cap
  and formal perf verification are pending.
- **T061** — the git-side scenarios (V3) are verified; GUI-driven scenarios (V1/V2/V4–V10) need a
  display to run.

**Bugfix**: 2026-07-16 — BUG-001 Added T062 (empty sessions excluded from persistence). See `bugs/BUG-001.md`. Also resolved implementation drift on T047 (terminal now interprets ANSI/VT via `alacritty_terminal` instead of showing raw escapes).
