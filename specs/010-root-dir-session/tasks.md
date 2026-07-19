# Tasks: Start a Session in the Project Root Directory

**Input**: Design documents from `/specs/010-root-dir-session/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md (all present)

**Tests**: Per Constitution Principle I (Test-First, NON-NEGOTIABLE), test tasks for all new
*pure/decision* logic (domain model, persistence, cwd resolution, filtering) are mandatory and
precede their implementation. Purely visual/rendering aspects (icon glyph, row styling) are
validated via `quickstart.md`, matching how features 008/009 treated their own analogous visual
work — documented per task below, not silently skipped.

**Documentation**: Per Constitution Principle VII, User Story 1 includes the user-guide update
in the same change (the "Default" concept it introduces is what the docs need to describe).

**Cross-platform**: Per Constitution Principle VI, no platform-specific code is introduced;
verified in Polish.

**Constitution dependency**: This feature depends on the v1.3.0 amendment to Principle III
(Native Worktree Integration) already made — a session may now map to the project root as the
sanctioned "Default" location.

> **Amendment (post-`/speckit-analyze` remediation, 2026-07-18)**: analysis found the original
> T009 undercounted the cwd-resolution call sites it must update (it said "four", the code has
> five) and named only 2 of the 5 functions — the two omitted, `session_has_conversation`
> (BUG-001 empty-session pruning) and `sync_session_titles`, are exactly the ones most likely to
> be missed since they aren't obviously "start/reopen" sites, and missing either breaks FR-009
> or FR-005 respectively. T009 below now enumerates all five explicitly. The original T012 also
> bundled its failing test and implementation into one task, unlike every sibling Foundational
> task; it is now split into T012/T013, shifting all subsequent task numbers up by one from the
> previously-reported task list. T014 (was T013) now also covers the "no project open" case
> (research.md/contracts' documented invariant had no test). See spec.md FR-007/FR-011 and the
> renamed Key Entity for the other remediations from that analysis.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no unresolved dependency)
- **[Story]**: US1/US2/US3 per spec.md

## Path Conventions

Single-project Rust + iced desktop app: `src/`, `tests/`, `docs/` at repository root (per
plan.md's Project Structure — no new directories).

---

## Phase 1: Setup

**Purpose**: Confirm no new dependency is required before touching the domain model.

- [X] T001 Confirm `Cargo.toml` needs no changes for this feature: `research.md`'s decisions
      (R1 `SessionLocation` enum, R3 `Option<String>` persistence widening, R6
      `Path::strip_prefix`) all use `std`/existing `serde`/`serde_json` only — no new crate.
      Record this as a one-line note in the PR description; no code change.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The `SessionLocation` domain model, its persistence, its cwd resolution, and the
generalized start message — every user story sits on top of this.

**⚠️ CRITICAL**: No user story task can begin until this phase is complete.

- [X] T002 [P] Write a failing test in `tests/session_lifecycle.rs` asserting
      `Session::start_new` and `Session::restored` take a `SessionLocation`
      (`Worktree(String)` | `Default`, data-model.md) instead of a bare `worktree_dir: impl
      Into<String>`, and that a session built with `SessionLocation::Default` carries no
      worktree identity — must fail to compile before T003.
- [X] T003 Add the `SessionLocation` enum and change `Session.worktree_dir: String` to
      `Session.location: SessionLocation` in `src/session.rs`; update `start_new`/`restored`
      signatures accordingly, **and update the module-level doc comment** (currently "A session
      is bound to one worktree (by `dir_name`)...") to describe both locations, so T002 passes.
- [X] T004 [P] Write a failing test in `tests/store_roundtrip.rs` (or extend
      `tests/session_store.rs`) asserting `StoredSession.worktree_dir` is `Option<String>`:
      `Some(dir)` round-trips to `SessionLocation::Worktree(dir)`, `None`/`null` round-trips to
      `SessionLocation::Default`, and a fixture JSON string with the OLD plain-string shape
      (`"worktree_dir": "feat-foo"`) still loads correctly with zero Default sessions inferred
      — must fail before T005.
- [X] T005 Widen `StoredSession.worktree_dir` from `String` to `Option<String>` in
      `src/store.rs`, and update the save/load mapping to/from `SessionLocation` per
      `contracts/storage-schema.md` so T004 passes.
- [X] T006 [P] Write a failing test covering `Workspace`/`State` call sites that today compare
      `s.worktree_dir == dir_name` (`running_session_count`, `find_session`,
      `worktree_tree`'s session filter) — assert they still correctly attribute sessions to
      their worktree after switching to `SessionLocation`, in `tests/workspace.rs` and
      `tests/sidebar_tree.rs` — must fail (fail to compile) before T007.
- [X] T007 Update the `worktree_dir`-comparison call sites in `src/workspace.rs` and
      `src/app.rs` (`app.rs` around the existing lines for `worktree_tree`,
      `running_session_count`'s callers, and the expansion-state lookups) to match on
      `SessionLocation::Worktree(dir_name)` instead of string equality — mechanical, no
      behavior change for existing worktree sessions — passes T006.
- [X] T008 [P] Write a failing test asserting the resolved cwd for a session with
      `SessionLocation::Default` equals the active project's root path exactly (no join/suffix),
      for **all five** call sites named in T009, in a new `tests/session_default_location.rs` —
      must fail before T009.
- [X] T009 Update **all five** cwd-resolution call sites in `src/main.rs` (today
      `repo.join(".claude/worktrees").join(&session.worktree_dir)` or equivalent) to branch on
      `session.location`: `Worktree(dir) => repo.join(".claude/worktrees").join(dir)`,
      `Default => repo.clone()` — passes T008 (research.md R2, which enumerates and names all
      five as the authoritative list). By function:
      1. `session_has_conversation` (`main.rs:316-318`) — BUG-001 empty-session pruning on load.
      2. The `Message::SessionStartRequested` handler (`main.rs:530-531`) — new session start.
      3. `sync_session_titles` (`main.rs:901-903`) — the title-sync poll.
      4. `session_cwd` (`main.rs:961`) — reopen/resume within the active project.
      5. `session_cwd_any` (`main.rs:971-972`) — reopen/resume across any project
         (background-restart crash-loop guard, feature 008 BS-6).
- [X] T010 [P] Write a failing test asserting that starting a `SessionLocation::Default` session
      calls zero worktree-mutation methods (`worktree_create`/`worktree_remove` and friends) on
      a `FakeGit`, in a new `tests/session_default_no_worktree.rs` — must fail before T011
      (data-model.md invariant, FR-002).
- [X] T011 Generalize `Message::SessionStartRequested { worktree_dir: String }` to
      `{ location: SessionLocation }` in `src/app.rs`, and update its `main.rs` handler so the
      `Default` arm never calls into `src/worktree.rs` — passes T010.
- [X] T012 [P] Write a failing test in `tests/icons.rs` asserting a new `Icon::ProjectRoot`
      variant exists with a codepoint looked up from the upstream Material Symbols
      `.codepoints` manifest for the `home` glyph (per `assets/fonts/PROVENANCE.md`'s "Adding a
      new icon" process — no font regeneration needed, the shipped font already has full
      coverage) and that `Icon::ALL.len()` increases by 1 — must fail before T013.
      Codepoint verified directly against the shipped font via `ttf_parser` (U+E88A resolves to
      a real glyph) rather than an unreachable upstream manifest fetch.
- [X] T013 Add the `Icon::ProjectRoot` variant + codepoint + `PROVENANCE.md` table row in
      `src/icons.rs` so T012 passes.

**Checkpoint**: `SessionLocation` exists, persists, resolves a cwd at all five real call sites,
and drives session start without touching worktrees. User story implementation can now begin.

---

## Phase 3: User Story 1 - Start a session in the project root without creating a worktree (Priority: P1) 🎯 MVP

**Goal**: The sidebar shows a permanent "Default" entry, present even with zero worktrees (and
absent when no project is open); starting a session from it runs in the project root and
creates no worktree.

**Independent Test**: Open a project with no worktrees, start a session from the Default entry,
confirm its shell's cwd is the project root and `.claude/worktrees/` gains no new directory.

### Tests for User Story 1

- [X] T014 [P] [US1] Write a failing test in `tests/sidebar_tree.rs` asserting the sidebar's
      entry-building function (a) always includes exactly one Default entry for an open project
      — even with zero worktrees — ahead of any worktree entries, and (b) produces **no**
      Default entry when no project is open (`workspace.active == None`) — the second case is
      the documented invariant in `contracts/sidebar-default-entry.md` §Invariants 1, previously
      untested (data-model.md `SidebarEntry`) — must fail before T016.
- [X] T015 [P] [US1] Write a failing test in `tests/app_state.rs` (or
      `tests/session_lifecycle.rs`) asserting that dispatching
      `Message::SessionStartRequested { location: SessionLocation::Default }` adds a new
      session with `SessionLocation::Default` to `Workspace.sessions` for the active project —
      must fail before T016/T017. Corrected during implementation: `SessionStartRequested` has
      no pure-reducer effect for any location (it's an I/O trigger only); the real assertion
      point is `Message::SessionStarted`, exercised identically to the existing
      `session_started_selected_and_closed` test — see the in-file comment on
      `default_session_started_enters_workspace_sessions`.

### Implementation for User Story 1

- [X] T016 [US1] Add the `SidebarEntry`/`DefaultNode` types and a `sidebar_entries()` builder in
      `src/app.rs` that prepends one `SidebarEntry::Default` ahead of the nodes
      `filtered_worktree_tree()` already returns, when a project is open (and produces none when
      it isn't), per `data-model.md` — passes T014/T015.
- [X] T017 [US1] In `src/ui/sidebar.rs`, render the Default row using the existing
      `TreeView`/tree-item row shape (reusing the existing `IconButton`/`Icon::AddSession`
      "start session" action, dispatching `Message::SessionStartRequested { location:
      SessionLocation::Default }`) — validated end-to-end via `quickstart.md` steps 2-3
      (`cargo run`). Simplification: the Default row's start-session action is always visible
      (not hover-fade-revealed like worktree rows) since it is the row's only action and there
      is only ever one Default row — no clutter/reflow concern motivates replicating the
      per-worktree hover-fade animation machinery for it.
- [X] T018 [US1] Add a "Default" section to `docs/user-guide/worktrees-and-sessions.md`
      describing the project-root entry point, how it differs from a worktree, and how to start
      a session from it (Constitution Principle VII — ships in the same change).

**Checkpoint**: User Story 1 is fully functional and independently testable — this is the MVP.
Run `cargo test` and `quickstart.md` steps 1-4 before moving on.

---

## Phase 4: User Story 2 - Distinguish root sessions from worktree sessions in the sidebar (Priority: P2)

**Goal**: The Default entry is visually distinct from worktree entries (its own icon, not
worktree-styled), is exempt from the sidebar's tag-filter panel (FR-011), and every entry —
Default or worktree — shows a hover tooltip with its location relative to the project (FR-010).

**Independent Test**: With both a Default entry and a worktree present, confirm they're visually
distinguishable at a glance, confirm the Default entry stays visible under any active tag
filter, and confirm hovering each shows the correct location tooltip.

### Tests for User Story 2

- [X] T019 [P] [US2] Write a failing test asserting the worktree-row tooltip text is computed as
      the worktree's path relative to the project root via `Path::strip_prefix(project_root)`
      (research.md R6), in `tests/sidebar_tree.rs` — must fail before T021.
- [X] T020 [P] [US2] Write a failing test asserting the Default entry remains present and
      visible in the sidebar's entry list regardless of which sidebar tag filters (feature 009)
      are active — including when `available_tag_filters()` is non-empty and one is toggled on
      — in `tests/sidebar_state.rs` (research.md R4, FR-011, `contracts/sidebar-default-entry.md`
      invariant 2) — must fail before T022.

### Implementation for User Story 2

- [X] T021 [US2] In `src/ui/sidebar.rs`: apply the new `Icon::ProjectRoot` (T013) to the Default
      row instead of any git/branch iconography; wrap every row (Default and worktree) with the
      existing `Tooltip::new(content, label, roles)` builder (`src/ui/material/mod.rs`) —
      worktree rows show the T019 relative-path text, the Default row shows a fixed "Project
      root" label. The icon/exact wording is visual-only, validated via `quickstart.md` step 7
      and the visual/asset check (matches how 008/009 treated pure icon/tint work) — passes
      T019 for the computed half. Implemented by adding a new `row_tooltip: Option<String>`
      field + `.row_tooltip(...)` builder method to the shared `TreeItem`/`TreeView` primitive
      (`src/ui/material/tree_view.rs`) — extending the reusable component (Principle VIII)
      rather than wrapping each call site ad hoc, mirroring the existing `trailing_tooltip`
      pattern already on `TreeItem`.
- [X] T022 [US2] In the `sidebar_entries()`/`filtered_worktree_tree()` composition (`src/app.rs`,
      T016), ensure tag filtering (`matches_filters`) is applied only to `SidebarEntry::Worktree`
      nodes — the `SidebarEntry::Default` node bypasses it unconditionally — passes T020. Already
      satisfied by T016's original implementation (`sidebar_entries()` filters only the worktree
      portion); no further change needed here, confirmed by T020 passing.

**Checkpoint**: User Stories 1 and 2 both work independently.

---

## Phase 5: User Story 3 - Run multiple concurrent root sessions (Priority: P3)

**Goal**: A project can have more than one Default session open at once, independently usable,
mirroring how one worktree already hosts multiple sessions today.

**Independent Test**: Start two sessions from the Default entry; confirm both stay open and
independently usable, and closing one does not affect the other.

> No new pure/decision logic is introduced here — `Workspace.sessions` is already a
> `Vec<Session>` per project with no cardinality limit per location (proven for worktrees since
> feature 005); `SessionLocation::Default` sessions use the exact same list, so multiplicity is
> already correct once Foundational + US1 land. This story is a regression lock, not new
> implementation — consistent with how feature 009 treated its own thin US2/US3 layers.

### Tests for User Story 3

- [X] T023 [P] [US3] Write a test in `tests/session_isolation.rs` asserting that starting a
      second `SessionLocation::Default` session for the same project succeeds, both are listed
      independently in `Workspace.sessions`, and stopping/closing one leaves the other's
      `SessionLifecycle` untouched (SC-005). This should already pass once T003–T011 land (no
      new coupling to add) — confirm it's genuinely exercising two simultaneous Default
      sessions, not one.

### Implementation for User Story 3

- [ ] T024 [US3] Manual validation per `quickstart.md` step 6: in `cargo run`, start two Default
      sessions and confirm both remain open and independently closable. **Not performed by the
      implementing agent** — this environment has no GUI-automation tooling, and launching the
      app would open a real window on the operator's live desktop session unprompted. Deferred
      to the user; T023's automated regression lock covers the same guarantee at the pure-core
      level.

**Checkpoint**: All three user stories are independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T025 [P] Run the existing worktree-only suites unmodified except for the mechanical
      `SessionLocation::Worktree(..)` construction change (`tests/worktree_create.rs`,
      `tests/worktree_delete.rs`, `tests/worktree_rollback.rs`, `tests/worktree_discovery.rs`)
      — confirm all still pass (FR-008 regression guard). 11/11 pass.
- [X] T026 [P] Add a `tests/store_roundtrip.rs` fixture using a `projects.json` shaped exactly
      as it was before this feature (plain string `worktree_dir` values only) and assert it
      loads with zero `SessionLocation::Default` sessions inferred (`contracts/storage-schema.md`
      backward-compatibility guarantee).
- [~] T027 [P] Add a manual `quickstart.md` step (and exercise it here) covering the edge case
      "project root directory becomes unavailable while a Default session is running": make the
      project root inaccessible (e.g. rename/unmount it) while a Default session is open, and
      confirm the session surfaces a failure/disconnected state consistent with a worktree
      session's behavior in the same situation (spec.md Edge Cases) — this was previously
      undocumented and unvalidated. The quickstart.md step was added (step 11); actually
      exercising it requires the GUI (see T024's note) — not performed by the implementing agent.
- [X] T028 Run the full `cargo test` suite (default `gui` feature and `--no-default-features`)
      and confirm no platform-specific code was introduced anywhere in this feature
      (Constitution Principle VI). 48/48 test binaries pass (0 failures); `git diff` confirms
      the only pre-existing `cfg(target_os = "linux")` in `src/main.rs` is untouched and no new
      one was added.
- [ ] T029 Run the full `quickstart.md` manual validation end-to-end via `cargo run` (all 11 GUI
      steps — including T027's new step — the visual/asset check, and the documentation check).
      **Not performed by the implementing agent** — no GUI-automation tooling in this
      environment (see T024's note); deferred to the user.
- [X] T030 [P] Review `README.md`'s one-line feature summary ("Open a git project and manage its
      worktrees...") and update it if it now reads as incomplete without mentioning the
      Default/root session option (per the constitution v1.3.0 amendment note, deliberately
      deferred from that change to here).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup (T001). Blocks all user stories.
- **User Story 1 (Phase 3)**: Depends on Foundational. No dependency on US2/US3.
- **User Story 2 (Phase 4)**: Depends on Foundational **and** on US1's `SidebarEntry`/Default
  row existing (T016/T017) — it adds an icon and tooltip to a row US1 creates, and filters the
  entry list US1 builds. Not independently buildable before US1, but independently
  *testable/demoable* once both are in place.
- **User Story 3 (Phase 5)**: Depends on Foundational and US1 (needs the Default entry to start
  sessions from). Behaviorally independent of US2.
- **Polish (Phase 6)**: Depends on all three user stories being complete.

### Within Each Phase

- Tests precede their implementation task (Constitution Principle I) — see each phase's task
  ordering above.
- T002→T003, T004→T005, T006→T007, T008→T009, T010→T011, T012→T013 all follow Red-Green (every
  Foundational pair, including the icon, is now split — no bundled Red+Green task remains).
- T014/T015→T016→T017; T019/T020→T021/T022; T023 is a regression lock (no paired implementation).

### Parallel Opportunities

- T002, T004, T006, T008, T010, T012 (distinct files/concerns within Foundational) can be
  written in parallel; keep each test's paired implementation task sequential relative to it.
- T014 and T015 (US1 tests, different files) can run in parallel.
- T019 and T020 (US2 tests, different files) can run in parallel.
- T025, T026, T027, T030 (Polish) are independent and parallelizable.

---

## Parallel Example: Foundational Phase

```bash
# Can run together (disjoint files/concerns):
Task: "Write failing SessionLocation/Session test in tests/session_lifecycle.rs"      # T002
Task: "Write failing StoredSession Option<String> test in tests/store_roundtrip.rs"   # T004
Task: "Write failing cwd-resolution test in tests/session_default_location.rs"        # T008
Task: "Write failing Icon::ProjectRoot test in tests/icons.rs"                        # T012
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 (Setup) and Phase 2 (Foundational) — `SessionLocation` exists end-to-end,
   correctly wired at all five real cwd-resolution call sites.
2. Complete Phase 3 (User Story 1) — delivers the feature's core ask: start a session in the
   project root without creating a worktree.
3. **STOP and VALIDATE**: run `cargo test` + `quickstart.md` steps 1-4.
4. This is a demoable MVP even without US2's visual distinction/tooltip polish or US3's
   multi-session confirmation (both already work mechanically once Foundational lands — US2/US3
   make that explicit and user-visible/regression-locked).

### Incremental Delivery

1. Setup + Foundational → `SessionLocation` domain model, persistence, and cwd resolution ready.
2. Add User Story 1 → test independently → MVP (Default entry + working session start).
3. Add User Story 2 → test independently → visual distinction + location tooltip ship.
4. Add User Story 3 → test independently → multi-session regression lock ships.
5. Polish → regression guards, unavailable-root edge case, cross-platform pass, docs review,
   final quickstart run.

---

## Notes

- [P] tasks touch different files with no unresolved dependency between them.
- US2 and US3 build ON TOP OF US1's `SidebarEntry`/Default row rather than being buildable
  before it — called out explicitly in Dependencies, mirroring how feature 009's US2/US3 built
  on its US1 primitive.
- No task manufactures a test for behavior with no decision logic behind it (icon glyph shape,
  exact tooltip wording) — those are called out as quickstart-validated instead.
- Principle VII's documentation gate is satisfied by T018 (US1) — no separate doc task is
  needed for US2/US3, matching feature 009's precedent.
