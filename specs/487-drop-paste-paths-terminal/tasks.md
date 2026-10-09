# Tasks: Drop Files or Paste Images into a Terminal to Insert Their Paths

**Input**: `specs/487-drop-paste-paths-terminal/` (plan.md, spec.md, data-model.md, contracts/terminal-insertion.md, research.md, quickstart.md)

**Tests**: Mandatory (Constitution I). In every phase the test task comes first, is run and seen to fail for the right reason, and only then is the implementation task done (`speckit-tdd-plan`).

**Documentation**: each story's user-guide update is in that story's phase (Constitution VII; CI's user-guide gate).

**Cross-platform**: quoting and planning stay in render-free `micold-core`; only the clipboard read and temp-root lookup are platform edges in the client `shell/` (Constitution VI). No wire or protocol change.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup and spikes

- [X] T001 Spike S1: confirm in `crates/micold-client/src/features/session.rs` and `crates/micold-client/src/app.rs` that `TerminalBytes` reaches the PTY of a non-focused pane (pane-addressed by `TerminalRef`); if it does not, add a pane-addressed variant. Record the finding in `specs/487-drop-paste-paths-terminal/research.md` (R7).
- [X] T002 Spike S2: confirm pane rectangles are obtainable for hit-testing from `crates/micold-client/src/ui/material/split_view.rs` (and `crates/micold-core/src/pane_layout.rs` `rects`); decide how the layout publishes them to app state. Record the finding in `research.md` (R1).
- [X] T003 Add the empty module skeleton `crates/micold-core/src/path_insert/{mod.rs,quote.rs}` (types only: `ShellKind`, `InsertTarget::Host`, `InsertedPath`, `Refusal`, `InsertionPlan`) and `pub mod path_insert` in `crates/micold-core/src/lib.rs`.

## Phase 2: Foundational (blocks all stories)

- [X] T004 [P] Write failing unit tests in `crates/micold-core/src/path_insert/quote.rs` (`#[cfg(test)]`): `quote(shell, name)` tables for every `ShellKind` over at least 20 awkward names (drive-letter and UNC paths, space, `'`, `"`, `$`, backtick, backslash, newline, tab, leading `-`, non-ASCII, `%`, `!`); `Unrepresentable` for Cmd and non-UTF-8 per R2 (SC-003, FR-002).
- [X] T005 Implement `ShellKind` and `quote` per shell in `crates/micold-core/src/path_insert/quote.rs` until T004 passes (`mise run test-core`).
- [X] T006 [P] Write failing tests in `crates/micold-core/src/terminal.rs` (`#[cfg(test)]`): `ShellKind::detect(shell command)` by basename; unknown → Posix on unix, Cmd on Windows.
- [X] T007 Implement `ShellKind::detect` beside `default_shell_command` in `crates/micold-core/src/terminal.rs` until T006 passes.
- [X] T008 [P] Write a failing real-shell round-trip test in `crates/micold-core/tests/path_insert_roundtrip.rs`: quoted text run by `sh`/`bash` (`printf %s`), and zsh and fish where installed, yields exactly the file name for the T004 names; each skipped when the shell is absent; cmd is covered by the T004 tables only; PowerShell on Windows CI (SC-003).

**Checkpoint**: quoting is proven for each shell.

## Phase 3: User Story 1 - Drop files to insert their paths (P1) 🎯 MVP

**Goal**: dropped files appear quoted at the pointed-at pane's input, never submitted (FR-001 to FR-005, FR-012 for exited or empty panes).

**Independent test**: drop two files, one `my file's (1).png`, onto a terminal: both quoted paths at the prompt, nothing executed.

- [X] T009 [P] [US1] Write failing tests in `crates/micold-core/src/path_insert/mod.rs` (`#[cfg(test)]`): `plan_insertion` with `InsertTarget::Host` keeps input order, joins with single spaces, `text()` is `None` when nothing is accepted, a directory is inserted like a file, a missing file is still accepted, an unrepresentable name lands in `refused` with the file named.
- [X] T010 [US1] Implement `plan_insertion`, `InsertionPlan::text` and `Refusal::message` for the host target in `crates/micold-core/src/path_insert/mod.rs` until T009 passes.
- [X] T011 [P] [US1] Write failing client reducer tests in `crates/micold-client/tests/features_session.rs`: `FilesDropped` → `Outcome::Insert` whose bytes (via `keymap::paste_bytes`, bracketed when requested) contain no `\r`/`\n` outside the bracket markers (SC-004); the target is the pane under the pointer, focused or not (US1.5); text already typed is untouched (no leading space); an exited process or empty pane → `Outcome::Notify` and no insert; a drop with no usable paths is silent; partial refusals notify.
- [X] T012 [P] [US1] Write failing client tests in `crates/micold-client/src/main_tests.rs`: pointer state follows `CursorMoved`; `window::Event::FileDropped` events of one update turn coalesce, in event order, into one `FilesDropped(Vec<PathBuf>)`.
- [X] T013 [P] [US1] Write a failing geometry-gate test in `crates/micold-client/tests/pane_drop_target.rs` (484's gate style): in a split layout the drop target resolves to the pane under the pointer, including the unfocused one, and outside every pane resolves to none.
- [X] T014 [US1] Add `Outcome::Insert { terminal, text }` in `crates/micold-client/src/features/mod.rs` and the `FilesDropped` / `InsertionFailed` messages with their reducer in `crates/micold-client/src/features/session.rs` (per T001's finding) until T011 passes.
- [X] T015 [US1] Publish pane rectangles from `crates/micold-client/src/ui/material/split_view.rs` and add the pointer state and hit-test in `crates/micold-client/src/app.rs`, until T013 passes.
- [X] T016 [US1] Add the pointer (`CursorMoved`) and file-drop subscriptions with per-turn coalescing in `crates/micold-client/src/shell/subscriptions.rs`, and route `Outcome::Insert` to `TerminalBytes` in `crates/micold-client/src/app.rs`, until T012 passes.
- [X] T017 [US1] Add the "Drop files and paste screenshots" section (drop part) to `docs/user-guide/terminal-panes.md`.

**Checkpoint**: dropping files works end to end on a regular terminal. M1 ships.

## Phase 4: User Story 2 - Paste a clipboard image into an AI session (P1)

**Goal**: a pasted image becomes a file in the worktree or session temp dir and its quoted path is inserted (FR-006 to FR-009, FR-012).

**Independent test**: put an image on the clipboard, paste into an AI session: a file with that image exists at the inserted path.

- [x] T018 [P] [US2] Write failing unit tests in `crates/micold-core/src/path_insert/pasted.rs` (`#[cfg(test)]`): `PastedLayout::dir` is `<worktree>/.micold-pasted/<session-id>` or `<data_dir>/pasted/<session-id>` when there is no worktree; `next_file(now, seq)` names are unique for two pastes in the same nanosecond; `exclude_line()` is `/.micold-pasted/`.
- [x] T019 [US2] Implement `PastedLayout` (`dir`, `next_file`, `exclude_line`) in `crates/micold-core/src/path_insert/pasted.rs` until T018 passes.
- [x] T020 [P] [US2] Write failing tests with tempdirs in `crates/micold-client/src/shell/pasted_image.rs` (`#[cfg(test)]`): RGBA → PNG file round-trips; the dir gets a `.gitignore` containing `*`; the `info/exclude` append (found by `git rev-parse --git-path info/exclude`) is idempotent and `git status` stays clean; a read-only worktree falls back to the temp dir; a write failure is an `Err` naming the cause.
- [x] T021 [US2] Add `arboard` (with `wayland-data-control`) and `png` to `crates/micold-client/Cargo.toml` and implement `crates/micold-client/src/shell/pasted_image.rs` (read, encode, write, exclude append) until T020 passes.
- [x] T022 [P] [US2] Write failing tests for a pure `paste_source(text, has_image, is_ai_session)` decision in `crates/micold-client/src/shell/clipboard.rs` (`#[cfg(test)]`): text alone or text plus image → Text (FR-009, US2.3); image alone in an AI session → Image; image alone in a regular terminal → Text/unchanged. And failing reducer tests in `crates/micold-client/tests/features_session.rs`: with empty clipboard text in an AI session `ImagePasted` → `Outcome::Insert` of the quoted path with no newline; text on the clipboard (alone or with an image) pastes as today and no file is created (FR-009); a regular terminal is unchanged; unreadable image or write failure → notice and no insert; two pastes give distinct paths (US2.5).
- [x] T023 [US2] Add the `ImagePasted` message and reducer in `crates/micold-client/src/features/session.rs` and `paste_source` plus the image branch in `on_paste_requested` in `crates/micold-client/src/shell/clipboard.rs` (only when the text read is empty), until T022 passes.
- [x] T024 [US2] Extend the "Drop files and paste screenshots" section of `docs/user-guide/terminal-panes.md` with the paste part.

**Checkpoint**: M2 ships.

## Phase 5: User Story 3 - Sandboxed sessions (P2)

**Goal**: sandboxed sessions get container paths; files the container cannot see are refused with a notice (FR-010, FR-011).

**Independent test**: in a sandboxed session drop a project file (container path inserted) and `~/outside.txt` (nothing inserted, notice names it).

- [x] T025 [P] [US3] Write failing unit tests in `crates/micold-core/src/path_insert/mod.rs`: `InsertTarget::Sandbox(&MountSet, mounted)` maps a project path through `pathmap::map_for`; a path outside every project → `Refusal::OutsideProjects`; a project not yet mounted → `NotMounted`; a symlink resolving outside is refused; a missing file is judged by its canonicalised parent; the shell is forced to `Bash`; a mixed drop accepts the visible ones and refuses the rest with messages naming them (US3.3).
- [x] T026 [US3] Implement the sandbox target of `plan_insertion` in `crates/micold-core/src/path_insert/mod.rs` (reusing `crates/micold-core/src/sandbox/pathmap.rs`) until T025 passes.
- [x] T027 [P] [US3] Write failing tests in `crates/micold-client/tests/features_session.rs` and `crates/micold-client/src/shell/pasted_image.rs`: a sandboxed drop and a sandboxed pasted image insert the container-side path; with an unwritable worktree the image goes to `MountSet.state` and its container path is inserted; if neither is possible nothing is inserted and a notice says why (US3.4, US3.5).
- [x] T028 [US3] Wire the sandbox target (session's `MountSet` and mounted list) into the drop and paste reducers in `crates/micold-client/src/features/session.rs` and the `MountSet.state` fallback in `crates/micold-client/src/shell/pasted_image.rs`, until T027 passes.
- [x] T029 [US3] Add the sandbox note to `docs/user-guide/sandboxed-daemon.md` and a one-line pointer from `docs/user-guide/terminal-panes.md`.

**Checkpoint**: M3 ships.

## Phase 6: User Story 4 - Temporary images are cleaned up (P2)

**Goal**: pasted images disappear with their session and after a crash; dropped files are never touched (FR-013, FR-014).

**Independent test**: paste an image, delete the session: the file is gone and a dropped file is untouched.

- [x] T030 [P] [US4] Write failing unit tests in `crates/micold-core/src/path_insert/pasted.rs`: `orphans(roots, live_sessions)` returns only `pasted/<id>` dirs of non-live sessions and `.micold-pasted/*` entries of non-live sessions, never a path outside a `pasted` root (SC-005).
- [x] T031 [US4] Implement `PastedLayout::orphans` in `crates/micold-core/src/path_insert/pasted.rs` until T030 passes.
- [x] T032 [P] [US4] Write failing daemon tests in `crates/micold-daemon/tests/pasted_cleanup.rs`: `delete_session` removes `<worktree>/.micold-pasted/<id>` and `<data|state>/pasted/<id>` and leaves a dropped file and other sessions' dirs; the start sweep removes `pasted/<dead-id>` and known worktrees' stale `.micold-pasted/*` (SC-005).
- [x] T033 [US4] Implement the removal in `delete_session` and the startup sweep in `crates/micold-daemon/src/state.rs` until T032 passes.
- [x] T034 [US4] Add the cleanup paragraph to `docs/user-guide/terminal-panes.md`.

**Checkpoint**: M4 ships.

## Dependencies and execution order

- Phase 1 spikes before T011/T013/T014/T015; T003 before Phase 2.
- Phase 2 blocks all stories. Within a story: tests, then implementation. `[P]` tasks touch disjoint files.
- US2 reuses US1's `Outcome::Insert`; US3 extends US1's `plan_insertion` and US2's image writer; US4 extends US2's `PastedLayout`.

## Implementation Strategy

MVP = Phases 1 to 3 (M1). Then US2, US3, US4, each as its own merge. The quickstart Part B visual pass changes no code and is done by the close unit.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Drop files to insert quoted paths 🎯 MVP

- **Tasks**: T001–T017
- **Deliverable**: dropping files on a terminal pane (focused or not) types their shell-quoted absolute paths at its input, nothing submitted.
- **Satisfies**: US1 acceptance scenarios 1–5; FR-001, FR-002, FR-003, FR-004, FR-005, FR-012 (drop part); SC-001, SC-003, SC-004
- **Verify**: `mise run test-core` (quoting tables, `plan_insertion`), `mise run gate` (reducer, coalescing, geometry gate, real-shell round trip); quickstart §B steps 1–2
- **Depends on**: —
- **Tier**: full

### M2 — Paste a clipboard image into an AI session

- **Tasks**: T018–T024
- **Deliverable**: pasting with an image and no text into an AI session saves a PNG in the worktree (hidden from `git status`) or the session temp dir and types its quoted path.
- **Satisfies**: US2 acceptance scenarios 1–5; FR-006, FR-007, FR-008, FR-009, FR-012; SC-002, SC-004
- **Verify**: `mise run test-core`, `mise run gate`; quickstart §B steps 3–4
- **Depends on**: M1
- **Tier**: full

### M3 — Sandboxed sessions insert container paths

- **Tasks**: T025–T029
- **Deliverable**: in a sandboxed session a dropped or pasted file is inserted as its container path, and a file outside the registered projects is refused with a notice naming it.
- **Satisfies**: US3 acceptance scenarios 1–5; FR-010, FR-011; SC-006
- **Verify**: `mise run test-core`, `mise run gate`; quickstart §B step 5
- **Depends on**: M1, M2
- **Tier**: full

### M4 — Pasted images are cleaned up

- **Tasks**: T030–T034
- **Deliverable**: deleting a session removes its pasted images, a restart sweeps orphans, and dropped files are never deleted.
- **Satisfies**: US4 acceptance scenarios 1–4; FR-013, FR-014; SC-005
- **Verify**: `mise run test-core`, `mise run gate`; quickstart §B step 6
- **Depends on**: M2
- **Tier**: full
